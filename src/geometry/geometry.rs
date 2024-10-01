use std::marker::PhantomData;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    visit::EdgeRef,
    Direction::Incoming,
};

use crate::{
    drawing::{
        bend::BendWeight,
        dot::DotWeight,
        graph::{PrimitiveWeight, Retag},
        primitive::Primitive,
        rules::AccessRules,
        seg::SegWeight,
    },
    geometry::{
        compound::ManageCompounds,
        primitive::{BendShape, DotShape, PrimitiveShape, SegShape},
    },
    graph::{GenericIndex, GetPetgraphIndex},
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
    Compound,
}

#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum GenericNode<P, C> {
    Primitive(P),
    Compound(C),
}

pub trait AccessDotWeight<CW>: GetPos + SetPos + GetWidth + Into<CW> + Copy {}
pub trait AccessSegWeight<CW>: GetWidth + Into<CW> + Copy {}
pub trait AccessBendWeight<CW>: GetOffset + SetOffset + GetWidth + Into<CW> + Copy {}

#[derive(Debug)]
pub struct Geometry<
    PW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: AccessDotWeight<PW>,
    SW: AccessSegWeight<PW>,
    BW: AccessBendWeight<PW>,
    CW: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
    DI: GetPetgraphIndex + Into<PI> + Copy,
    SI: GetPetgraphIndex + Into<PI> + Copy,
    BI: GetPetgraphIndex + Into<PI> + Copy,
> {
    graph: StableDiGraph<GenericNode<PW, CW>, GeometryLabel, usize>,
    weight_marker: PhantomData<PW>,
    dot_weight_marker: PhantomData<DW>,
    seg_weight_marker: PhantomData<SW>,
    bend_weight_marker: PhantomData<BW>,
    compound_weight_marker: PhantomData<CW>,
    index_marker: PhantomData<PI>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

impl<
        PW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight<PW>,
        SW: AccessSegWeight<PW>,
        BW: AccessBendWeight<PW>,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
        DI: GetPetgraphIndex + Into<PI> + Copy,
        SI: GetPetgraphIndex + Into<PI> + Copy,
        BI: GetPetgraphIndex + Into<PI> + Copy,
    > Geometry<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::default(),
            weight_marker: PhantomData,
            dot_weight_marker: PhantomData,
            seg_weight_marker: PhantomData,
            bend_weight_marker: PhantomData,
            compound_weight_marker: PhantomData,
            index_marker: PhantomData,
            dot_index_marker: PhantomData,
            seg_index_marker: PhantomData,
            bend_index_marker: PhantomData,
        }
    }

    pub fn add_dot<W: AccessDotWeight<PW>>(&mut self, weight: W) -> GenericIndex<W> {
        GenericIndex::<W>::new(self.graph.add_node(GenericNode::Primitive(weight.into())))
    }

    pub fn add_seg<W: AccessSegWeight<PW>>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let seg =
            GenericIndex::<W>::new(self.graph.add_node(GenericNode::Primitive(weight.into())));

        self.graph.update_edge(
            from.petgraph_index(),
            seg.petgraph_index(),
            GeometryLabel::Joined,
        );
        self.graph.update_edge(
            seg.petgraph_index(),
            to.petgraph_index(),
            GeometryLabel::Joined,
        );

        seg
    }

    pub fn add_bend<W: AccessBendWeight<PW>>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let bend =
            GenericIndex::<W>::new(self.graph.add_node(GenericNode::Primitive(weight.into())));

        self.graph.update_edge(
            from.petgraph_index(),
            bend.petgraph_index(),
            GeometryLabel::Joined,
        );
        self.graph.update_edge(
            bend.petgraph_index(),
            to.petgraph_index(),
            GeometryLabel::Joined,
        );
        self.graph.update_edge(
            bend.petgraph_index(),
            core.petgraph_index(),
            GeometryLabel::Core,
        );

        bend
    }

    pub fn remove_primitive(&mut self, primitive: PI) {
        self.graph.remove_node(primitive.petgraph_index());
    }

    pub fn move_dot(&mut self, dot: DI, to: Point) {
        let mut weight = self.dot_weight(dot);
        weight.set_pos(to);
        *self.graph.node_weight_mut(dot.petgraph_index()).unwrap() =
            GenericNode::Primitive(weight.into());
    }

    pub fn shift_bend(&mut self, bend: BI, offset: f64) {
        let mut weight = self.bend_weight(bend);
        weight.set_offset(offset);
        *self.graph.node_weight_mut(bend.petgraph_index()).unwrap() =
            GenericNode::Primitive(weight.into());
    }

    pub fn flip_bend(&mut self, bend: BI) {
        let (from, to) = self.bend_joints(bend);
        let from_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(from.petgraph_index(), bend.petgraph_index())
                    .unwrap(),
            )
            .unwrap();
        let to_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(bend.petgraph_index(), to.petgraph_index())
                    .unwrap(),
            )
            .unwrap();
        self.graph
            .update_edge(from.petgraph_index(), bend.petgraph_index(), to_edge_weight);
        self.graph
            .update_edge(bend.petgraph_index(), to.petgraph_index(), from_edge_weight);
    }

    pub fn reattach_bend(&mut self, bend: BI, maybe_new_inner: Option<BI>) {
        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.petgraph_index(), Incoming)
            .find(|edge| *edge.weight() == GeometryLabel::Outer)
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        if let Some(new_inner) = maybe_new_inner {
            self.graph.update_edge(
                new_inner.petgraph_index(),
                bend.petgraph_index(),
                GeometryLabel::Outer,
            );
        }
    }

    pub fn dot_shape(&self, dot: DI) -> PrimitiveShape {
        let weight = self.dot_weight(dot);
        PrimitiveShape::Dot(DotShape {
            circle: Circle {
                pos: weight.pos(),
                r: weight.width() / 2.0,
            },
        })
    }

    pub fn seg_shape(&self, seg: SI) -> PrimitiveShape {
        let (from, to) = self.seg_joints(seg);
        PrimitiveShape::Seg(SegShape {
            from: self.dot_weight(from).pos(),
            to: self.dot_weight(to).pos(),
            width: self.primitive_weight(seg.petgraph_index()).width(),
        })
    }

    pub fn bend_shape(&self, bend: BI) -> PrimitiveShape {
        let (from, to) = self.bend_joints(bend);
        let core_weight = self.core_weight(bend);
        PrimitiveShape::Bend(BendShape {
            from: self.dot_weight(from).pos(),
            to: self.dot_weight(to).pos(),
            inner_circle: Circle {
                pos: core_weight.pos(),
                r: self.inner_radius(bend),
            },
            width: self.primitive_weight(bend.petgraph_index()).width(),
        })
    }

    fn inner_radius(&self, bend: BI) -> f64 {
        let mut r = self.bend_weight(bend).offset();
        let mut rail = bend;

        while let Some(inner) = self.inner(rail) {
            let weight: BW = self.bend_weight(inner);
            r += weight.width() + weight.offset();
            rail = inner;
        }

        self.core_weight(bend).width() / 2.0 + r
    }

    fn primitive_weight(&self, index: NodeIndex<usize>) -> PW {
        if let GenericNode::Primitive(weight) = *self.graph.node_weight(index).unwrap() {
            weight
        } else {
            unreachable!()
        }
    }

    pub fn dot_weight(&self, dot: DI) -> DW {
        self.primitive_weight(dot.petgraph_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn seg_weight(&self, seg: SI) -> SW {
        self.primitive_weight(seg.petgraph_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn bend_weight(&self, bend: BI) -> BW {
        self.primitive_weight(bend.petgraph_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> CW {
        if let GenericNode::Compound(weight) =
            *self.graph.node_weight(compound.petgraph_index()).unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn core_weight(&self, bend: BI) -> DW {
        self.graph
            .neighbors(bend.petgraph_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.petgraph_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| {
                self.primitive_weight(ni)
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
                self.primitive_weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn core(&self, bend: BI) -> DI {
        self.graph
            .neighbors(bend.petgraph_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.petgraph_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| {
                self.primitive_weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
            .unwrap()
    }

    pub fn inner(&self, bend: BI) -> Option<BI> {
        self.graph
            .neighbors_directed(bend.petgraph_index(), Incoming)
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(*ni, bend.petgraph_index()).unwrap())
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|ni| {
                self.primitive_weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn outer(&self, bend: BI) -> Option<BI> {
        self.graph
            .neighbors(bend.petgraph_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.petgraph_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|ni| {
                self.primitive_weight(ni)
                    .retag(ni)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
    }

    pub fn joineds(&self, node: PI) -> impl Iterator<Item = PI> + '_ {
        self.graph
            .neighbors_undirected(node.petgraph_index())
            .filter(move |ni| {
                matches!(
                    self.graph
                        .edge_weight(
                            self.graph
                                .find_edge_undirected(node.petgraph_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Joined
                )
            })
            .map(|ni| self.primitive_weight(ni).retag(ni))
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

    pub fn compound_members(&self, compound: GenericIndex<CW>) -> impl Iterator<Item = PI> + '_ {
        self.graph
            .neighbors_directed(compound.petgraph_index(), Incoming)
            .filter(move |ni| {
                matches!(
                    self.graph
                        .edge_weight(
                            self.graph
                                .find_edge(*ni, compound.petgraph_index())
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Compound
                )
            })
            .map(|ni| self.primitive_weight(ni).retag(ni))
    }

    pub fn graph(&self) -> &StableDiGraph<GenericNode<PW, CW>, GeometryLabel, usize> {
        &self.graph
    }
}

impl<
        PW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight<PW>,
        SW: AccessSegWeight<PW>,
        BW: AccessBendWeight<PW>,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
        DI: GetPetgraphIndex + Into<PI> + Copy,
        SI: GetPetgraphIndex + Into<PI> + Copy,
        BI: GetPetgraphIndex + Into<PI> + Copy,
    > ManageCompounds<CW, GenericIndex<CW>> for Geometry<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW> {
        GenericIndex::<CW>::new(self.graph.add_node(GenericNode::Compound(weight)))
    }

    fn remove_compound(&mut self, compound: GenericIndex<CW>) {
        self.graph.remove_node(compound.petgraph_index());
    }

    fn add_to_compound<W>(&mut self, primitive: GenericIndex<W>, compound: GenericIndex<CW>) {
        self.graph.update_edge(
            primitive.petgraph_index(),
            compound.petgraph_index(),
            GeometryLabel::Compound,
        );
    }

    fn compound_weight(&self, compound: GenericIndex<CW>) -> CW {
        if let GenericNode::Compound(weight) =
            *self.graph.node_weight(compound.petgraph_index()).unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn compounds<W>(&self, node: GenericIndex<W>) -> impl Iterator<Item = GenericIndex<CW>> {
        self.graph
            .neighbors(node.petgraph_index())
            .filter(move |ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(node.petgraph_index(), *ni).unwrap())
                        .unwrap(),
                    GeometryLabel::Compound
                )
            })
            .map(|ni| GenericIndex::new(ni))
    }
}
