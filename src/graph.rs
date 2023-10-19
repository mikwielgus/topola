use enum_as_inner::EnumAsInner;
use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use std::marker::PhantomData;

use crate::math::Circle;

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

#[enum_dispatch(Retag)]
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

#[enum_dispatch(GetNodeIndex)]
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

macro_rules! untag {
    ($index:ident, $expr:expr) => {
        match $index {
            Index::Dot($index) => $expr,
            Index::Seg($index) => $expr,
            Index::Bend($index) => $expr,
        }
    };
}

pub type DotIndex = GenericIndex<DotWeight>;
pub type SegIndex = GenericIndex<SegWeight>;
pub type BendIndex = GenericIndex<BendWeight>;
