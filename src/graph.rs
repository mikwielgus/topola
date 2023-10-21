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

pub trait Ends<Start, Stop> {
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

#[enum_dispatch(Retag, GetNet)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Weight {
    Dot(DotWeight),
    Seg(SegWeight),
    Bend(BendWeight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl Retag for DotWeight {
    fn retag(&self, index: NodeIndex<usize>) -> Index {
        Index::Dot(DotIndex {
            node_index: index,
            marker: PhantomData,
        })
    }
}

impl GetNet for DotWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegWeight {
    pub net: i64,
    pub width: f64,
}

impl Retag for SegWeight {
    fn retag(&self, index: NodeIndex<usize>) -> Index {
        Index::Seg(SegIndex {
            node_index: index,
            marker: PhantomData,
        })
    }
}

impl GetNet for SegWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendWeight {
    pub net: i64,
    pub cw: bool,
}

impl Retag for BendWeight {
    fn retag(&self, index: NodeIndex<usize>) -> Index {
        Index::Bend(BendIndex {
            node_index: index,
            marker: PhantomData,
        })
    }
}

impl GetNet for BendWeight {
    fn net(&self) -> i64 {
        self.net
    }
}

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Label {
    End,
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

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Index {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
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

pub type DotIndex = GenericIndex<DotWeight>;

impl MakePrimitive for DotIndex {
    fn primitive<'a>(&self, graph: &'a StableDiGraph<Weight, Label, usize>) -> Primitive<'a> {
        Primitive::Dot(GenericPrimitive::new(*self, graph))
    }
}

pub type SegIndex = GenericIndex<SegWeight>;

impl MakePrimitive for SegIndex {
    fn primitive<'a>(&self, graph: &'a StableDiGraph<Weight, Label, usize>) -> Primitive<'a> {
        Primitive::Seg(GenericPrimitive::new(*self, graph))
    }
}

pub type BendIndex = GenericIndex<BendWeight>;

impl MakePrimitive for BendIndex {
    fn primitive<'a>(&self, graph: &'a StableDiGraph<Weight, Label, usize>) -> Primitive<'a> {
        Primitive::Bend(GenericPrimitive::new(*self, graph))
    }
}
