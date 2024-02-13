use std::marker::PhantomData;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    visit::EdgeRef,
    Direction::{Incoming, Outgoing},
};

use crate::{
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        bend::{BendWeight, FixedBendWeight, LooseBendWeight},
        dot::{DotWeight, FixedDotWeight, LooseDotWeight},
        geometry::shape::{BendShape, DotShape, SegShape, Shape},
        graph::{GeometryWeight, Retag},
    },
    math::Circle,
};

#[enum_dispatch]
pub trait GetPos {
    fn pos(&self) -> Point;
}

#[enum_dispatch]
pub trait SetPos {
    fn set_pos(&mut self, pos: Point);
}

#[enum_dispatch]
pub trait GetWidth {
    fn width(&self) -> f64;
}

#[enum_dispatch]
pub trait GetOffset {
    fn offset(&self) -> f64;
}

#[enum_dispatch]
pub trait SetOffset {
    fn set_offset(&mut self, offset: f64);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryLabel {
    Joined,
    Outer,
    Core,
}

pub trait DotWeightTrait<GW>: GetPos + SetPos + GetWidth + Into<GW> + Copy {}
pub trait SegWeightTrait<GW>: GetWidth + Into<GW> + Copy {}
pub trait BendWeightTrait<GW>: GetOffset + SetOffset + GetWidth + Into<GW> + Copy {}

#[derive(Debug)]
pub struct Geometry<
    GW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<GI> + Copy,
    DW: DotWeightTrait<GW> + Copy,
    SW: SegWeightTrait<GW> + Copy,
    BW: BendWeightTrait<GW> + Copy,
    GI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
    DI: GetNodeIndex + Into<GI> + Copy,
    SI: GetNodeIndex + Into<GI> + Copy,
    BI: GetNodeIndex + Into<GI> + Copy,
> {
    graph: StableDiGraph<GW, GeometryLabel, usize>,
    weight_marker: PhantomData<GW>,
    dot_weight_marker: PhantomData<DW>,
    seg_weight_marker: PhantomData<SW>,
    bend_weight_marker: PhantomData<BW>,
    index_marker: PhantomData<GI>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

impl<
        GW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<GI> + Copy,
        DW: DotWeightTrait<GW> + Copy,
        SW: SegWeightTrait<GW> + Copy,
        BW: BendWeightTrait<GW> + Copy,
        GI: GetNodeIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
        DI: GetNodeIndex + Into<GI> + Copy,
        SI: GetNodeIndex + Into<GI> + Copy,
        BI: GetNodeIndex + Into<GI> + Copy,
    > Geometry<GW, DW, SW, BW, GI, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::default(),
            weight_marker: PhantomData,
            dot_weight_marker: PhantomData,
            seg_weight_marker: PhantomData,
            bend_weight_marker: PhantomData,
            index_marker: PhantomData,
            dot_index_marker: PhantomData,
            seg_index_marker: PhantomData,
            bend_index_marker: PhantomData,
        }
    }

    pub fn add_dot<W: DotWeightTrait<GW>>(&mut self, weight: W) -> GenericIndex<W> {
        GenericIndex::<W>::new(self.graph.add_node(weight.into()))
    }

    pub fn add_seg<W: SegWeightTrait<GW>>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let seg = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .update_edge(from.node_index(), seg.node_index(), GeometryLabel::Joined);
        self.graph
            .update_edge(seg.node_index(), to.node_index(), GeometryLabel::Joined);

        seg
    }

    pub fn add_bend<W: BendWeightTrait<GW>>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let bend = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .update_edge(from.node_index(), bend.node_index(), GeometryLabel::Joined);
        self.graph
            .update_edge(bend.node_index(), to.node_index(), GeometryLabel::Joined);
        self.graph
            .update_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);

        bend
    }

    pub fn remove(&mut self, node: GI) {
        self.graph.remove_node(node.node_index());
    }

    pub fn move_dot(&mut self, dot: DI, to: Point) {
        let mut weight = self.dot_weight(dot);
        weight.set_pos(to);
        *self.graph.node_weight_mut(dot.node_index()).unwrap() = weight.into();
    }

    pub fn shift_bend(&mut self, bend: BI, offset: f64) {
        let mut weight = self.bend_weight(bend);
        weight.set_offset(offset);
        *self.graph.node_weight_mut(bend.node_index()).unwrap() = weight.into();
    }

    pub fn flip_bend(&mut self, bend: BI) {
        let (from, to) = self.bend_joints(bend);
        let from_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(from.node_index(), bend.node_index())
                    .unwrap(),
            )
            .unwrap();
        let to_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(bend.node_index(), to.node_index())
                    .unwrap(),
            )
            .unwrap();
        self.graph
            .update_edge(from.node_index(), bend.node_index(), to_edge_weight);
        self.graph
            .update_edge(bend.node_index(), to.node_index(), from_edge_weight);
    }

    pub fn reattach_bend(&mut self, bend: BI, maybe_new_inner: Option<BI>) {
        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.node_index(), Incoming)
            .filter(|edge| *edge.weight() == GeometryLabel::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        if let Some(new_inner) = maybe_new_inner {
            self.graph.update_edge(
                new_inner.node_index(),
                bend.node_index(),
                GeometryLabel::Outer,
            );
        }
    }

    pub fn dot_shape(&self, dot: DI) -> Shape {
        let weight = self.dot_weight(dot);
        Shape::Dot(DotShape {
            c: Circle {
                pos: weight.pos(),
                r: weight.width() / 2.0,
            },
        })
    }

    pub fn seg_shape(&self, seg: SI) -> Shape {
        let (from, to) = self.seg_joints(seg);
        Shape::Seg(SegShape {
            from: self.dot_weight(from).pos(),
            to: self.dot_weight(to).pos(),
            width: self.weight(seg.node_index()).width(),
        })
    }

    pub fn bend_shape(&self, bend: BI) -> Shape {
        let (from, to) = self.bend_joints(bend);
        let core_weight = self.core_weight(bend);
        Shape::Bend(BendShape {
            from: self.dot_weight(from).pos(),
            to: self.dot_weight(to).pos(),
            c: Circle {
                pos: core_weight.pos(),
                r: self.inner_radius(bend),
            },
            width: self.weight(bend.node_index()).width(),
        })
    }

    fn inner_radius(&self, bend: BI) -> f64 {
        let mut r = self.bend_weight(bend).offset();
        let mut rail = bend;

        while let Some(inner) = self.inner(rail) {
            let weight: BW = self
                .bend_weight(inner)
                .try_into()
                .unwrap_or_else(|_| unreachable!());
            r += weight.width() + weight.offset();
            rail = inner;
        }

        self.core_weight(bend).width() / 2.0 + r
    }

    fn weight(&self, index: NodeIndex<usize>) -> GW {
        *self.graph.node_weight(index).unwrap()
    }

    pub fn dot_weight(&self, dot: DI) -> DW {
        self.weight(dot.node_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn seg_weight(&self, seg: SI) -> SW {
        self.weight(seg.node_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn bend_weight(&self, bend: BI) -> BW {
        self.weight(bend.node_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    fn core_weight(&self, bend: BI) -> DW {
        self.graph
            .neighbors(bend.node_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.node_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
            .unwrap()
    }

    pub fn first_rail(&self, node: NodeIndex<usize>) -> Option<BI> {
        self.graph
            .neighbors_directed(node, Incoming)
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(*ni, node).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn core(&self, bend: BI) -> DI {
        self.graph
            .neighbors(bend.node_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.node_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
            .unwrap()
    }

    pub fn inner(&self, bend: BI) -> Option<BI> {
        self.graph
            .neighbors_directed(bend.node_index(), Incoming)
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(*ni, bend.node_index()).unwrap())
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn outer(&self, bend: BI) -> Option<BI> {
        self.graph
            .neighbors_directed(bend.node_index(), Outgoing)
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.node_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn joineds(&self, node: GI) -> impl Iterator<Item = GI> + '_ {
        self.graph
            .neighbors_undirected(node.node_index())
            .filter(move |ni| {
                matches!(
                    self.graph
                        .edge_weight(
                            self.graph
                                .find_edge_undirected(node.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Joined
                )
            })
            .map(|ni| {
                self.weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
    }

    pub fn seg_joints(&self, seg: SI) -> (DI, DI) {
        let v: Vec<_> = self.joineds(seg.into()).collect();
        (
            v[0].try_into().unwrap_or_else(|_| unreachable!()),
            v[1].try_into().unwrap_or_else(|_| unreachable!()),
        )
    }

    pub fn bend_joints(&self, bend: BI) -> (DI, DI) {
        let v: Vec<_> = self.joineds(bend.into()).collect();
        (
            v[0].try_into().unwrap_or_else(|_| unreachable!()),
            v[1].try_into().unwrap_or_else(|_| unreachable!()),
        )
    }

    pub fn joined_segs(&self, dot: DI) -> impl Iterator<Item = SI> + '_ {
        self.joineds(dot.into()).filter_map(|ni| ni.try_into().ok())
    }

    pub fn joined_bends(&self, dot: DI) -> impl Iterator<Item = BI> + '_ {
        self.joineds(dot.into()).filter_map(|ni| ni.try_into().ok())
    }

    pub fn graph(&self) -> &StableDiGraph<GW, GeometryLabel, usize> {
        &self.graph
    }
}
