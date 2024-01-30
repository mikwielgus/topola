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
        connectivity::{BandIndex, ComponentIndex},
        Layout,
    },
    math::Circle,
    primitive::Primitive,
    shape::{BendShape, DotShape, SegShape, Shape},
};

use super::super::{
    bend::{FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
        SeqLooseSegWeight,
    },
};

#[enum_dispatch]
pub trait Retag<GeometryIndex> {
    fn retag(&self, index: NodeIndex<usize>) -> GeometryIndex;
}

#[enum_dispatch]
pub trait GetComponentIndex {
    fn component(&self) -> ComponentIndex;
}

pub trait GetComponentIndexMut {
    fn component_mut(&mut self) -> &mut ComponentIndex;
}

pub trait GetBandIndex {
    fn band(&self) -> BandIndex;
}

#[enum_dispatch]
pub trait GetPos {
    fn pos(&self) -> Point;
}

#[enum_dispatch]
pub trait GetWidth {
    fn width(&self) -> f64;
}

#[enum_dispatch]
pub trait GetOffset {
    fn offset(&self) -> f64;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag<GeometryIndex> for $weight_struct {
            fn retag(&self, index: NodeIndex<usize>) -> GeometryIndex {
                GeometryIndex::$weight_variant($index_struct::new(index))
            }
        }

        pub type $index_struct = GenericIndex<$weight_struct>;

        impl MakePrimitive for $index_struct {
            fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a> {
                Primitive::$weight_variant(GenericPrimitive::new(*self, layout))
            }
        }
    };
}

macro_rules! impl_fixed_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);

        impl GetComponentIndex for $weight_struct {
            fn component(&self) -> ComponentIndex {
                self.component
            }
        }

        impl GetComponentIndexMut for $weight_struct {
            fn component_mut(&mut self) -> &mut ComponentIndex {
                &mut self.component
            }
        }
    };
}

macro_rules! impl_loose_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);

        impl GetBandIndex for $weight_struct {
            fn band(&self) -> BandIndex {
                self.band
            }
        }
    };
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryIndex {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LoneLooseSeg(LoneLooseSegIndex),
    SeqLooseSeg(SeqLooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetWidth, Retag<GeometryIndex>)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryWeight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    LoneLooseSeg(LoneLooseSegWeight),
    SeqLooseSeg(SeqLooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryLabel {
    Connection,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a>;
}

pub trait DotWeightTrait<GW>: GetPos + GetWidth + Into<GW> + Copy {}
pub trait SegWeightTrait<GW>: GetWidth + Into<GW> + Copy {}
pub trait BendWeightTrait<GW>: GetOffset + GetWidth + Into<GW> + Copy {}

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
    pub graph: StableDiGraph<GW, GeometryLabel, usize>,
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

        self.graph.update_edge(
            from.node_index(),
            seg.node_index(),
            GeometryLabel::Connection,
        );
        self.graph
            .update_edge(seg.node_index(), to.node_index(), GeometryLabel::Connection);

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

        self.graph.update_edge(
            from.node_index(),
            bend.node_index(),
            GeometryLabel::Connection,
        );
        self.graph.update_edge(
            bend.node_index(),
            to.node_index(),
            GeometryLabel::Connection,
        );
        self.graph
            .update_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);

        bend
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

    pub fn connecteds(&self, node: GI) -> impl Iterator<Item = GI> + '_ {
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
                    GeometryLabel::Connection
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
        let v: Vec<_> = self.connecteds(seg.into()).collect();
        (
            v[0].try_into().unwrap_or_else(|_| unreachable!()),
            v[1].try_into().unwrap_or_else(|_| unreachable!()),
        )
    }

    pub fn bend_joints(&self, bend: BI) -> (DI, DI) {
        let v: Vec<_> = self.connecteds(bend.into()).collect();
        (
            v[0].try_into().unwrap_or_else(|_| unreachable!()),
            v[1].try_into().unwrap_or_else(|_| unreachable!()),
        )
    }

    pub fn connected_segs(&self, dot: DI) -> impl Iterator<Item = SI> + '_ {
        self.connecteds(dot.into())
            .filter_map(|ni| ni.try_into().ok())
    }

    pub fn connected_bends(&self, dot: DI) -> impl Iterator<Item = BI> + '_ {
        self.connecteds(dot.into())
            .filter_map(|ni| ni.try_into().ok())
    }

    pub fn graph(&self) -> &StableDiGraph<GW, GeometryLabel, usize> {
        &self.graph
    }
}
