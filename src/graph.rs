use enum_as_inner::EnumAsInner;
use petgraph::stable_graph::NodeIndex;
use std::marker::PhantomData;

use crate::math::Circle;

pub trait Walk {
    fn interior(&self) -> Vec<TaggedIndex>;
    fn closure(&self) -> Vec<TaggedIndex>;
    fn ends(&self) -> [DotIndex; 2];
}

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum TaggedWeight {
    Dot(DotWeight),
    Seg(SegWeight),
    Bend(BendWeight),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotWeight {
    pub net: i64,
    pub circle: Circle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendWeight {
    pub net: i64,
    pub cw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegWeight {
    pub net: i64,
    pub width: f64,
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

    pub fn retag(&self, weight: &TaggedWeight) -> TaggedIndex {
        match weight {
            TaggedWeight::Dot(..) => TaggedIndex::Dot(DotIndex {
                index: self.index,
                marker: PhantomData,
            }),
            TaggedWeight::Seg(..) => TaggedIndex::Seg(SegIndex {
                index: self.index,
                marker: PhantomData,
            }),
            TaggedWeight::Bend(..) => TaggedIndex::Bend(BendIndex {
                index: self.index,
                marker: PhantomData,
            }),
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
