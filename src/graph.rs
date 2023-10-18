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
    fn retag(&self, index: NodeIndex<usize>) -> TaggedIndex;
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
    fn retag(&self, index: NodeIndex<usize>) -> TaggedIndex {
        TaggedIndex::Dot(DotIndex {
            index,
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
    fn retag(&self, index: NodeIndex<usize>) -> TaggedIndex {
        TaggedIndex::Seg(SegIndex {
            index,
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
    fn retag(&self, index: NodeIndex<usize>) -> TaggedIndex {
        TaggedIndex::Bend(BendIndex {
            index,
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

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum TaggedIndex {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Index<T> {
    pub index: NodeIndex<usize>,
    marker: PhantomData<T>,
}

impl<T> Index<T> {
    pub fn new(index: NodeIndex<usize>) -> Self {
        Self {
            index,
            marker: PhantomData,
        }
    }
}

pub trait Tag {
    fn tag(&self) -> TaggedIndex;
}

macro_rules! untag {
    ($index:ident, $expr:expr) => {
        match $index {
            TaggedIndex::Dot($index) => $expr,
            TaggedIndex::Seg($index) => $expr,
            TaggedIndex::Bend($index) => $expr,
        }
    };
}

pub type DotIndex = Index<DotWeight>;

impl Tag for DotIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::Dot(*self)
    }
}

pub type SegIndex = Index<SegWeight>;

impl Tag for SegIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::Seg(*self)
    }
}

pub type BendIndex = Index<BendWeight>;

impl Tag for BendIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::Bend(*self)
    }
}
