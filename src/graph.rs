use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::{
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

pub trait Interior<T> {
    fn interior(&self) -> Vec<T>;
}

pub trait GetEnds<F, T> {
    fn ends(&self) -> (F, T);
}

#[enum_dispatch]
pub trait Retag {
    fn retag(&self, index: NodeIndex<usize>) -> Index;
}

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

#[enum_dispatch]
pub trait GetNetMut {
    fn net_mut(&mut self) -> &mut i64;
}

#[enum_dispatch]
pub trait GetWidth {
    fn width(&self) -> f64;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag for $weight_struct {
            fn retag(&self, index: NodeIndex<usize>) -> Index {
                Index::$weight_variant($index_struct {
                    node_index: index,
                    marker: PhantomData,
                })
            }
        }

        impl GetNet for $weight_struct {
            fn net(&self) -> i64 {
                self.net
            }
        }

        impl GetNetMut for $weight_struct {
            fn net_mut(&mut self) -> &mut i64 {
                &mut self.net
            }
        }

        pub type $index_struct = GenericIndex<$weight_struct>;

        impl MakePrimitive for $index_struct {
            fn primitive<'a>(
                &self,
                graph: &'a StableDiGraph<Weight, Label, usize>,
            ) -> Primitive<'a> {
                Primitive::$weight_variant(GenericPrimitive::new(*self, graph))
            }
        }
    };
}

#[enum_dispatch(Retag, GetNet, GetNetMut)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Weight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    LooseSeg(LooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Index {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LooseSeg(LooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    Loose(LooseSegIndex),
}

impl From<SegIndex> for Index {
    fn from(seg: SegIndex) -> Self {
        match seg {
            SegIndex::Fixed(seg) => Index::FixedSeg(seg),
            SegIndex::Loose(seg) => Index::LooseSeg(seg),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendIndex {
    Fixed(FixedBendIndex),
    Loose(LooseBendIndex),
}

impl From<BendIndex> for Index {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => Index::FixedBend(bend),
            BendIndex::Loose(bend) => Index::LooseBend(bend),
        }
    }
}

pub trait DotWeight: GetNet + GetWidth + Into<Weight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_weight!(FixedDotWeight, FixedDot, FixedDotIndex);
impl DotWeight for FixedDotWeight {}

impl GetWidth for FixedDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_weight!(LooseDotWeight, LooseDot, LooseDotIndex);
impl DotWeight for LooseDotWeight {}

impl GetWidth for LooseDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

pub trait SegWeight: GetNet + Into<Weight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub net: i64,
    pub width: f64,
}

impl_weight!(FixedSegWeight, FixedSeg, FixedSegIndex);
impl SegWeight for FixedSegWeight {}

impl GetWidth for FixedSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseSegWeight {
    pub net: i64,
}

impl_weight!(LooseSegWeight, LooseSeg, LooseSegIndex);
impl SegWeight for LooseSegWeight {}

pub trait BendWeight: GetNet + Into<Weight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub net: i64,
    pub width: f64,
    pub cw: bool,
}

impl_weight!(FixedBendWeight, FixedBend, FixedBendIndex);
impl BendWeight for FixedBendWeight {}

impl GetWidth for FixedBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub net: i64,
    pub cw: bool,
}

impl_weight!(LooseBendWeight, LooseBend, LooseBendIndex);
impl BendWeight for LooseBendWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy)]
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

impl<W> Hash for GenericIndex<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_index.hash(state)
    }
}

impl<W> PartialEq for GenericIndex<W> {
    fn eq(&self, other: &Self) -> bool {
        self.node_index == other.node_index
    }
}

impl<W> Eq for GenericIndex<W> {}

impl<W> GetNodeIndex for GenericIndex<W> {
    fn node_index(&self) -> NodeIndex<usize> {
        self.node_index
    }
}
