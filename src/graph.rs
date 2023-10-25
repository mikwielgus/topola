use enum_as_inner::EnumAsInner;
use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use std::marker::PhantomData;

use crate::{
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

pub trait Interior<T> {
    fn interior(&self) -> Vec<T>;
}

pub trait GetEnds<Start, Stop> {
    fn ends(&self) -> (Start, Stop);
}

#[enum_dispatch]
pub trait Retag {
    fn retag(&self, index: NodeIndex<usize>) -> Index;
}

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

macro_rules! impl_type {
    ($weight_struct_name:ident, $weight_variant_name:ident, $index_struct_name:ident) => {
        impl Retag for $weight_struct_name {
            fn retag(&self, index: NodeIndex<usize>) -> Index {
                Index::$weight_variant_name($index_struct_name {
                    node_index: index,
                    marker: PhantomData,
                })
            }
        }

        impl GetNet for $weight_struct_name {
            fn net(&self) -> i64 {
                self.net
            }
        }

        pub type $index_struct_name = GenericIndex<$weight_struct_name>;

        impl MakePrimitive for $index_struct_name {
            fn primitive<'a>(
                &self,
                graph: &'a StableDiGraph<Weight, Label, usize>,
            ) -> Primitive<'a> {
                Primitive::$weight_variant_name(GenericPrimitive::new(*self, graph))
            }
        }
    };
}

#[enum_dispatch(Retag, GetNet)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Weight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    HalfLooseSeg(HalfLooseSegWeight),
    FullyLooseSeg(FullyLooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Index {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    HalfLooseSeg(HalfLooseSegIndex),
    FullyLooseSeg(FullyLooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum DotIndex {
    Fixed(FixedDotIndex),
    Loose(LooseDotIndex),
}

impl From<DotIndex> for Index {
    fn from(dot: DotIndex) -> Self {
        match dot {
            DotIndex::Fixed(fixed) => Index::FixedDot(fixed),
            DotIndex::Loose(loose) => Index::LooseDot(loose),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    HalfLoose(HalfLooseSegIndex),
    FullyLoose(FullyLooseSegIndex),
}

impl From<SegIndex> for Index {
    fn from(seg: SegIndex) -> Self {
        match seg {
            SegIndex::Fixed(fixed) => Index::FixedSeg(fixed),
            SegIndex::HalfLoose(half_loose) => Index::HalfLooseSeg(half_loose),
            SegIndex::FullyLoose(fully_loose) => Index::FullyLooseSeg(fully_loose),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum LooseSegIndex {
    Half(HalfLooseSegIndex),
    Fully(FullyLooseSegIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum BendIndex {
    Fixed(FixedBendIndex),
    Loose(LooseBendIndex),
}

impl From<BendIndex> for Index {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(fixed) => Index::FixedBend(fixed),
            BendIndex::Loose(loose) => Index::LooseBend(loose),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_type!(FixedDotWeight, FixedDot, FixedDotIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_type!(LooseDotWeight, LooseDot, LooseDotIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub net: i64,
    pub width: f64,
}

impl_type!(FixedSegWeight, FixedSeg, FixedSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfLooseSegWeight {
    pub net: i64,
    pub width: f64,
}

impl_type!(HalfLooseSegWeight, HalfLooseSeg, HalfLooseSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FullyLooseSegWeight {
    pub net: i64,
    pub width: f64,
}

impl_type!(FullyLooseSegWeight, FullyLooseSeg, FullyLooseSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub net: i64,
    pub cw: bool,
}

impl_type!(FixedBendWeight, FixedBend, FixedBendIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub net: i64,
    pub cw: bool,
}

impl_type!(LooseBendWeight, LooseBend, LooseBendIndex);

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Label {
    Adjacent,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait GetNodeIndex {
    fn node_index(&self) -> NodeIndex<usize>;
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, graph: &'a StableDiGraph<Weight, Label, usize>) -> Primitive<'a>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenericIndex<W> {
    node_index: NodeIndex<usize>,
    marker: PhantomData<W>,
}

impl<W> GenericIndex<W> {
    pub fn new(index: NodeIndex<usize>) -> Self {
        Self {
            node_index: index,
            marker: PhantomData,
        }
    }
}

impl<W> GetNodeIndex for GenericIndex<W> {
    fn node_index(&self) -> NodeIndex<usize> {
        self.node_index
    }
}
