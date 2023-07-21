use std::marker::PhantomData;
use enum_as_inner::EnumAsInner;
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
        return Mesh {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot = DotIndex::new(self.graph.add_node(TaggedWeight::Dot(weight)));
        let index = TaggedIndex::Dot(dot);
        self.rtree.insert(RTreeWrapper::new(self.shape(index), index));
        dot
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.shape(TaggedIndex::Dot(dot)), TaggedIndex::Dot(dot)));
        self.graph.remove_node(dot.index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg = SegIndex::new(self.graph.add_node(TaggedWeight::Seg(weight)));
        self.graph.add_edge(seg.index, from.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        let index = TaggedIndex::Seg(seg);
        self.rtree.insert(RTreeWrapper::new(self.shape(index), index));
        seg

    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.shape(TaggedIndex::Seg(seg)), TaggedIndex::Seg(seg)));
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

        let index = TaggedIndex::Bend(bend);
        self.rtree.insert(RTreeWrapper::new(self.shape(index), index));
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
        self.rtree.remove(&RTreeWrapper::new(self.shape(TaggedIndex::Bend(bend)), TaggedIndex::Bend(bend)));
        self.graph.remove_node(bend.index);
    }

    pub fn shapes(&self) -> Box<dyn Iterator<Item=Shape> + '_> {
        Box::new(self.rtree.iter().map(|wrapper| self.shape(wrapper.data)))
    }

    pub fn shape(&self, index: TaggedIndex) -> Shape {
        Shape {
            weight: match index {
                TaggedIndex::Dot(index) => self.primitive(index).tagged_weight(),
                TaggedIndex::Seg(index) => self.primitive(index).tagged_weight(),
                TaggedIndex::Bend(index) => self.primitive(index).tagged_weight(),
            },
            dot_neighbor_weights: match index {
                TaggedIndex::Dot(index) => {
                    self.primitive(index).ends()
                        .into_iter()
                        .map(|index| self.primitive(index).weight())
                        .collect()
                },
                TaggedIndex::Seg(index) => {
                    self.primitive(index).ends()
                        .into_iter()
                        .map(|index| self.primitive(index).weight())
                        .collect()
                },
                TaggedIndex::Bend(index) => {
                    self.primitive(index).ends()
                        .into_iter()
                        .map(|index| self.primitive(index).weight())
                        .collect()
                },
            },
            core_pos: match index {
                TaggedIndex::Bend(bend) => {
                    Some(self.primitive(self.primitive(bend).core()).weight().circle.pos)
                },
                _ => None,
            },
        }
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    /*pub fn tagged_weight(&self, index: TaggedIndex) -> Weight {
        match index {
            TaggedIndex::Dot(DotIndex {index: node, ..})
            | TaggedIndex::Seg(SegIndex {index: node, ..})
            | TaggedIndex::Bend(BendIndex {index: node, ..}) =>
                *self.graph.node_weight(node).unwrap(),
        }
    }*/
}
