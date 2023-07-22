use std::marker::PhantomData;
use enum_as_inner::EnumAsInner;
use geo::Point;
use petgraph::Direction::{Outgoing, Incoming};
use petgraph::stable_graph::{StableDiGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use rstar::{RTree, RTreeObject, AABB};
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::shape::Shape;
use crate::weight::{TaggedWeight, DotWeight, SegWeight, BendWeight, Label};


#[derive(Debug, Copy, Clone, PartialEq)]
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
    }
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

#[derive(Debug, EnumAsInner, Copy, Clone, PartialEq)]
pub enum TaggedIndex {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
}

pub type RTreeWrapper = GeomWithData<Shape, TaggedIndex>;

pub struct Mesh {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<TaggedWeight, Label, usize>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot = DotIndex::new(self.graph.add_node(TaggedWeight::Dot(weight)));
        self.rtree.insert(RTreeWrapper::new(self.primitive(dot).shape(), TaggedIndex::Dot(dot)));
        dot
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(dot).shape(), TaggedIndex::Dot(dot)));
        self.graph.remove_node(dot.index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg = SegIndex::new(self.graph.add_node(TaggedWeight::Seg(weight)));
        self.graph.add_edge(seg.index, from.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        self.rtree.insert(RTreeWrapper::new(self.primitive(seg).shape(), TaggedIndex::Seg(seg)));
        seg
    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(seg).shape(), TaggedIndex::Seg(seg)));
        self.graph.remove_node(seg.index);
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, around: TaggedIndex, weight: BendWeight) -> BendIndex {
        match around {
            TaggedIndex::Dot(core) =>
                self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) =>
                self.add_outer_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
        }
    }

    pub fn add_core_bend(&mut self, from: DotIndex, to: DotIndex, core: DotIndex, weight: BendWeight) -> BendIndex {
        let bend = BendIndex::new(self.graph.add_node(TaggedWeight::Bend(weight)));
        self.graph.add_edge(bend.index, from.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        self.rtree.insert(RTreeWrapper::new(self.primitive(bend).shape(), TaggedIndex::Bend(bend)));
        bend
    }

    pub fn add_outer_bend(&mut self, from: DotIndex, to: DotIndex, inner: BendIndex, weight: BendWeight) -> BendIndex {
        let core = *self.graph.neighbors(inner.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(inner.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();
        let bend = self.add_core_bend(from, to, core, weight);
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
        bend
    }

    pub fn remove_bend(&mut self, bend: BendIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(bend).shape(), TaggedIndex::Bend(bend)));
        self.graph.remove_node(bend.index);
    }

    /*pub fn extend_bend(&mut self, bend: BendIndex, to: Point) -> DotIndex {
        
    }

    pub fn shift_bend(&mut self, bend: BendIndex, offset: f64) {

    }*/

    pub fn nodes(&self) -> impl Iterator<Item=TaggedIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    /*fn insert_into_rtree<Weight>(&mut self, index: Index<Weight>) {
        self.rtree.insert(RTreeWrapper::new(self.primitive(index).shape(), index.tag()));
    }*/
}
