use contracts::debug_invariant;
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::{GenericIndex, GetNodeIndex},
    layout::Layout,
    primitive::Primitive,
};

use super::{
    bend::{BendWeight, FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegWeight,
        SeqLooseSegIndex, SeqLooseSegWeight,
    },
};

#[enum_dispatch]
pub trait Retag {
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
pub trait GetWidth {
    fn width(&self) -> f64;
}

pub trait GetOffset {
    fn offset(&self) -> f64;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag for $weight_struct {
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

pub type GeometryGraph = StableDiGraph<GeometryWeight, GeometryLabel, usize>;

#[enum_dispatch(Retag)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryLabel {
    Adjacent,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a>;
}

#[derive(Debug)]
pub struct Geometry {
    pub graph: GeometryGraph,
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::default(),
        }
    }

    pub fn add_dot<W: DotWeight>(&mut self, weight: W) -> GenericIndex<W> {
        GenericIndex::<W>::new(self.graph.add_node(weight.into()))
    }

    pub fn add_seg<W: SegWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        weight: W,
    ) -> GenericIndex<W> {
        let seg = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .update_edge(from.node_index(), seg.node_index(), GeometryLabel::Adjacent);
        self.graph
            .update_edge(seg.node_index(), to.node_index(), GeometryLabel::Adjacent);

        seg
    }

    pub fn add_bend<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        core: impl GetNodeIndex,
        weight: W,
    ) -> GenericIndex<W> {
        let bend = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph.update_edge(
            from.node_index(),
            bend.node_index(),
            GeometryLabel::Adjacent,
        );
        self.graph
            .update_edge(bend.node_index(), to.node_index(), GeometryLabel::Adjacent);
        self.graph
            .update_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);

        bend
    }

    pub fn graph(&self) -> &GeometryGraph {
        &self.graph
    }
}
