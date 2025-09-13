// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::marker::PhantomData;
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    stable_graph::StableDiGraph,
    visit::{EdgeRef, Walker},
    Direction::{Incoming, Outgoing},
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::{
    drawing::{
        bend::{BendWeight, FixedBendWeight, LooseBendWeight},
        dot::{DotWeight, FixedDotWeight, LooseDotWeight},
        graph::PrimitiveWeight,
        primitive::PrimitiveRef,
        seg::{FixedSegWeight, LoneLooseSegWeight, SegWeight, SeqLooseSegWeight},
    },
    geometry::{
        compound::ManageCompounds,
        primitive::{BendShape, DotShape, PrimitiveShape, SegShape},
    },
    graph::{GenericIndex, GetIndex},
    math::Circle,
};

pub trait Retag {
    type Index: Sized + GetIndex + PartialEq + Copy;
    fn retag(&self, index: usize) -> Self::Index;
}

#[enum_dispatch]
pub trait GetLayer {
    fn layer(&self) -> usize;
}

#[enum_dispatch]
pub trait GetSetPos {
    fn pos(&self) -> Point;
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
pub trait SetOffset: GetOffset {
    fn set_offset(&mut self, offset: f64);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GeometryLabel<Cel> {
    Joined,
    Outer,
    Core,
    Compound(Cel),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum GenericNode<P, C> {
    Primitive(P),
    Compound(C),
}

impl<P: GetIndex, C: GetIndex> GetIndex for GenericNode<P, C> {
    fn index(&self) -> usize {
        match self {
            Self::Primitive(x) => x.index(),
            Self::Compound(x) => x.index(),
        }
    }
}

pub trait AccessDotWeight: GetSetPos + GetWidth + Copy {}
impl<T: GetSetPos + GetWidth + Copy> AccessDotWeight for T {}

pub trait AccessSegWeight: GetWidth + Copy {}
impl<T: GetWidth + Copy> AccessSegWeight for T {}

pub trait AccessBendWeight: SetOffset + GetWidth + Copy {}
impl<T: SetOffset + GetWidth + Copy> AccessBendWeight for T {}

#[derive(Debug)]
pub struct Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
    graph: StableDiGraph<GenericNode<PW, CW>, GeometryLabel<Cel>, usize>,
    primitive_weight_marker: PhantomData<PW>,
    dot_weight_marker: PhantomData<DW>,
    seg_weight_marker: PhantomData<SW>,
    bend_weight_marker: PhantomData<BW>,
    compound_weight_marker: PhantomData<CW>,
    index_marker: PhantomData<PI>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

impl<PW: Clone, DW, SW, BW, CW: Clone, Cel: Clone, PI, DI, SI, BI> Clone
    for Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn clone(&self) -> Self {
        Self {
            graph: self.graph.clone(),
            ..Self::new()
        }
    }
}

impl<PW, DW, SW, BW, CW, Cel: Clone, PI, DI, SI, BI> Default
    for Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::default(),
            primitive_weight_marker: PhantomData,
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

    // we could use `derive_getters` to generate these, but `Geometry` only wraps a single
    // field that actually contains data...

    #[inline(always)]
    pub fn graph(&self) -> &StableDiGraph<GenericNode<PW, CW>, GeometryLabel<Cel>, usize> {
        &self.graph
    }

    fn primitive_weight(&self, index: usize) -> PW
    where
        PW: Copy,
    {
        if let GenericNode::Primitive(weight) = self.graph.node_weight(index.into()).unwrap() {
            *weight
        } else {
            unreachable!()
        }
    }
}

impl<
        PW: GetWidth + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<Index = PI> + Copy,
        DW: AccessDotWeight + Into<PW>,
        SW: AccessSegWeight + Into<PW>,
        BW: AccessBendWeight + Into<PW>,
        CW,
        Cel,
        PI: GetIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Copy,
        DI: GetIndex + Into<PI> + Copy,
        SI: GetIndex + Into<PI> + Copy,
        BI: GetIndex + Into<PI> + Copy,
    > Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn add_dot<W: AccessDotWeight + Into<PW>>(&mut self, weight: W) -> GenericIndex<W> {
        GenericIndex::<W>::new(
            self.graph
                .add_node(GenericNode::Primitive(weight.into()))
                .index(),
        )
    }

    pub(super) fn add_dot_at_index<W: AccessDotWeight + Into<PW>>(
        &mut self,
        dot: GenericIndex<W>,
        weight: W,
    ) {
        self.graph
            .update_node(dot.index().into(), GenericNode::Primitive(weight.into()));
    }

    pub fn add_seg<W: AccessSegWeight + Into<PW>>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let seg = GenericIndex::<W>::new(
            self.graph
                .add_node(GenericNode::Primitive(weight.into()))
                .index(),
        );
        self.init_seg_joints(seg, from, to);
        seg
    }

    pub(super) fn add_seg_at_index<W: AccessSegWeight + Into<PW>>(
        &mut self,
        seg: GenericIndex<W>,
        from: DI,
        to: DI,
        weight: W,
    ) {
        self.graph
            .update_node(seg.index().into(), GenericNode::Primitive(weight.into()));
        self.init_seg_joints(seg, from, to);
    }

    fn init_seg_joints<W: AccessSegWeight + Into<PW>>(
        &mut self,
        seg: GenericIndex<W>,
        from: DI,
        to: DI,
    ) {
        self.graph.update_edge(
            from.index().into(),
            seg.index().into(),
            GeometryLabel::Joined,
        );
        self.graph
            .update_edge(seg.index().into(), to.index().into(), GeometryLabel::Joined);
    }

    pub fn is_joined_with<I>(&self, seg: I, node: GenericNode<PI, GenericIndex<CW>>) -> bool
    where
        I: Copy + GetIndex,
        CW: Clone,
        Cel: Copy,
    {
        match node {
            GenericNode::Primitive(prim) => self
                .graph
                .find_edge_undirected(seg.index().into(), prim.index().into())
                .map_or(false, |(eidx, _direction)| {
                    matches!(self.graph.edge_weight(eidx).unwrap(), GeometryLabel::Joined)
                }),
            GenericNode::Compound(comp) => self
                .compound_members(comp)
                .any(|(_cel, i)| self.is_joined_with(seg, GenericNode::Primitive(i))),
        }
    }

    pub fn add_bend<W: AccessBendWeight + Into<PW>>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let bend = GenericIndex::<W>::new(
            self.graph
                .add_node(GenericNode::Primitive(weight.into()))
                .index(),
        );
        self.init_bend_joints_and_core(bend, from, to, core);
        bend
    }

    pub(super) fn add_bend_at_index<W: AccessBendWeight + Into<PW>>(
        &mut self,
        bend: GenericIndex<W>,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) {
        self.graph
            .update_node(bend.index().into(), GenericNode::Primitive(weight.into()));
        self.init_bend_joints_and_core(bend, from, to, core);
    }

    pub(super) fn add_compound_at_index(&mut self, compound: GenericIndex<CW>, weight: CW) {
        self.graph
            .update_node(compound.index().into(), GenericNode::Compound(weight));
    }

    fn init_bend_joints_and_core<W: AccessBendWeight + Into<PW>>(
        &mut self,
        bend: GenericIndex<W>,
        from: DI,
        to: DI,
        core: DI,
    ) {
        self.graph.update_edge(
            from.index().into(),
            bend.index().into(),
            GeometryLabel::Joined,
        );
        self.graph.update_edge(
            bend.index().into(),
            to.index().into(),
            GeometryLabel::Joined,
        );
        self.graph.update_edge(
            bend.index().into(),
            core.index().into(),
            GeometryLabel::Core,
        );
    }

    pub fn remove_primitive(&mut self, primitive: PI) {
        debug_assert!(self.graph.remove_node(primitive.index().into()).is_some());
    }

    pub fn move_dot(&mut self, dot: DI, to: Point) {
        let mut weight = self.dot_weight(dot);
        weight.set_pos(to);
        *self.graph.node_weight_mut(dot.index().into()).unwrap() =
            GenericNode::Primitive(weight.into());
    }

    pub fn shift_bend(&mut self, bend: BI, offset: f64) {
        let mut weight = self.bend_weight(bend);
        weight.set_offset(offset);
        *self.graph.node_weight_mut(bend.index().into()).unwrap() =
            GenericNode::Primitive(weight.into());
    }

    pub fn flip_bend(&mut self, bend: BI) {
        let (from, to) = self.bend_joints(bend);
        let from_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(from.index().into(), bend.index().into())
                    .unwrap(),
            )
            .unwrap();
        let to_edge_weight = self
            .graph
            .remove_edge(
                self.graph
                    .find_edge(bend.index().into(), to.index().into())
                    .unwrap(),
            )
            .unwrap();
        self.graph
            .update_edge(from.index().into(), bend.index().into(), to_edge_weight);
        self.graph
            .update_edge(bend.index().into(), to.index().into(), from_edge_weight);
    }

    pub fn reattach_bend(&mut self, bend: BI, maybe_new_inner: Option<BI>) {
        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.index().into(), Incoming)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Outer))
        {
            debug_assert!(self.graph.remove_edge(old_inner_edge.id()).is_some());
        }

        if let Some(new_inner) = maybe_new_inner {
            self.graph.update_edge(
                new_inner.index().into(),
                bend.index().into(),
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
            width: self.primitive_weight(seg.index().into()).width(),
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
            width: self.primitive_weight(bend.index().into()).width(),
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

    pub fn dot_weight(&self, dot: DI) -> DW {
        self.primitive_weight(dot.index().into())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn seg_weight(&self, seg: SI) -> SW {
        self.primitive_weight(seg.index().into())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn bend_weight(&self, bend: BI) -> BW {
        self.primitive_weight(bend.index().into())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> &CW {
        if let GenericNode::Compound(weight) =
            self.graph.node_weight(compound.index().into()).unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn core_weight(&self, bend: BI) -> DW {
        self.graph
            .edges_directed(bend.index().into(), Outgoing)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Core))
            .map(|edge| {
                self.primitive_weight(edge.target().index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .unwrap()
    }

    pub fn joineds(&self, node: PI) -> impl Iterator<Item = PI> + '_ {
        self.graph
            .neighbors_undirected(node.index().into())
            .filter(move |ni| {
                matches!(
                    self.graph
                        .edge_weight(
                            self.graph
                                .find_edge_undirected(node.index().into(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Joined
                )
            })
            .map(|ni| self.primitive_index(ni.index()))
    }

    pub fn joined_segs(&self, dot: DI) -> impl Iterator<Item = SI> + '_ {
        self.joineds(dot.into()).filter_map(|ni| ni.try_into().ok())
    }

    pub fn joined_bends(&self, dot: DI) -> impl Iterator<Item = BI> + '_ {
        self.joineds(dot.into()).filter_map(|ni| ni.try_into().ok())
    }

    fn joints(&self, node: PI) -> (DI, DI) {
        let lhs = self
            .graph
            .edges_directed(node.index().into(), Incoming)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Joined))
            .map(|edge| edge.source());
        let rhs = self
            .graph
            .edges_directed(node.index().into(), Outgoing)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Joined))
            .map(|edge| edge.target());

        (
            lhs.map(|ni| {
                self.primitive_index(ni.index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .unwrap(),
            rhs.map(|ni| {
                self.primitive_index(ni.index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .unwrap(),
        )
    }

    pub fn seg_joints(&self, seg: SI) -> (DI, DI) {
        self.joints(seg.into())
    }

    pub fn bend_joints(&self, bend: BI) -> (DI, DI) {
        self.joints(bend.into())
    }
}

impl<PW: Copy + Retag<Index = PI>, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
    Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn primitive_index(&self, index: usize) -> PI {
        self.primitive_weight(index).retag(index)
    }
}

pub struct OutwardWalker<BI> {
    frontier: VecDeque<BI>,
}

impl<BI: GetIndex> OutwardWalker<BI> {
    pub fn new(initial_frontier: impl Iterator<Item = BI>) -> Self {
        let mut frontier = VecDeque::new();
        frontier.extend(initial_frontier);

        Self { frontier }
    }
}

impl<
        PW: Copy + Retag<Index = PI>,
        DW,
        SW,
        BW,
        CW,
        Cel,
        PI: TryInto<DI> + TryInto<SI> + TryInto<BI>,
        DI,
        SI,
        BI: Copy + GetIndex,
    > Walker<&Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>> for OutwardWalker<BI>
{
    type Item = BI;

    fn walk_next(
        &mut self,
        geometry: &Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>,
    ) -> Option<Self::Item> {
        let front = self.frontier.pop_front()?;
        self.frontier.extend(geometry.outers(front));

        Some(front)
    }
}

impl<
        PW: Copy + Retag<Index = PI>,
        DW,
        SW,
        BW,
        CW,
        Cel,
        PI: TryInto<DI> + TryInto<SI> + TryInto<BI>,
        DI,
        SI,
        BI: GetIndex,
    > Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn all_rails(&self, index: usize) -> impl Iterator<Item = BI> + '_ {
        self.graph
            .edges_directed(index.into(), Incoming)
            .filter(|edge| matches!(edge.weight(), GeometryLabel::Core))
            .map(|edge| {
                self.primitive_index(edge.source().index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
    }

    pub fn core(&self, bend: BI) -> DI {
        self.graph
            .edges_directed(bend.index().into(), Outgoing)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Core))
            .map(|edge| {
                self.primitive_index(edge.target().index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .unwrap()
    }

    pub fn inner(&self, bend: BI) -> Option<BI> {
        self.graph
            .edges_directed(bend.index().into(), Incoming)
            .find(|edge| matches!(edge.weight(), GeometryLabel::Outer))
            .map(|edge| {
                self.primitive_index(edge.source().index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
    }

    pub fn outers(&self, bend: BI) -> impl Iterator<Item = BI> + '_ {
        self.graph
            .edges_directed(bend.index().into(), Outgoing)
            .filter(|edge| matches!(edge.weight(), GeometryLabel::Outer))
            .map(|edge| {
                self.primitive_index(edge.target().index())
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
    }

    pub fn outwards(&self, bend: BI) -> OutwardWalker<BI> {
        OutwardWalker::new(self.outers(bend))
    }
}

impl<PW: Copy + Retag<Index = PI>, DW, SW, BW, CW: Clone, Cel: Copy, PI: Copy, DI, SI, BI>
    ManageCompounds<CW> for Geometry<PW, DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    type GeneralIndex = PI;
    type EntryLabel = Cel;

    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW> {
        GenericIndex::<CW>::new(self.graph.add_node(GenericNode::Compound(weight)).index())
    }

    fn remove_compound(&mut self, compound: GenericIndex<CW>) {
        debug_assert!(self.graph.remove_node(compound.index().into()).is_some());
    }

    fn add_to_compound<I>(&mut self, primitive: I, entry_label: Cel, compound: GenericIndex<CW>)
    where
        I: Copy + GetIndex,
    {
        self.graph.update_edge(
            primitive.index().into(),
            compound.index().into(),
            GeometryLabel::Compound(entry_label),
        );
    }

    fn compound_weight(&self, compound: GenericIndex<CW>) -> &CW {
        if let GenericNode::Compound(weight) =
            self.graph.node_weight(compound.index().into()).unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn compound_members(
        &self,
        compound: GenericIndex<CW>,
    ) -> impl Iterator<Item = (Cel, Self::GeneralIndex)> + '_ {
        self.graph
            .edges_directed(compound.index().into(), Incoming)
            .filter_map(|edge| {
                if let GeometryLabel::Compound(entry_label) = *edge.weight() {
                    Some((entry_label, self.primitive_index(edge.source().index())))
                } else {
                    None
                }
            })
    }

    fn compounds<I>(&self, node: I) -> impl Iterator<Item = (Cel, GenericIndex<CW>)>
    where
        I: Copy + GetIndex,
    {
        self.graph
            .edges_directed(node.index().into(), Outgoing)
            .filter_map(|edge| {
                if let GeometryLabel::Compound(entry_label) = *edge.weight() {
                    Some((entry_label, GenericIndex::new(edge.target().index())))
                } else {
                    None
                }
            })
    }
}
