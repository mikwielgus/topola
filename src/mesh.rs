use std::marker::PhantomData;
use enum_as_inner::EnumAsInner;
use petgraph::Direction::{Outgoing, Incoming};
use petgraph::stable_graph::{StableDiGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use rstar::{RTree, RTreeObject, AABB};
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::weight::{Weight, DotWeight, SegWeight, BendWeight, Label};


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Index<Ix, T> {
    index: Ix,
    marker: PhantomData<T>,
}

impl<Ix, T> Index<Ix, T> {
    pub fn new(index: Ix) -> Self {
        Self {
            index,
            marker: PhantomData,
        }
    }
}

pub trait Tag {
    fn tag(&self) -> TaggedIndex;
}

pub type DotIndex = Index<NodeIndex<usize>, DotWeight>;

impl Tag for DotIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::Dot(*self)
    }
}

pub type SegIndex = Index<NodeIndex<usize>, SegWeight>;

impl Tag for SegIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::Seg(*self)
    }
}

pub type BendIndex = Index<NodeIndex<usize>, BendWeight>;

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

pub type RTreeWrapper = GeomWithData<Primitive, TaggedIndex>;

pub struct Mesh {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<Weight, Label, usize>,
}

impl Mesh {
    pub fn new() -> Self {
        return Mesh {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot = DotIndex::new(self.graph.add_node(Weight::Dot(weight)));
        let index = TaggedIndex::Dot(dot);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
        dot
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(TaggedIndex::Dot(dot)), TaggedIndex::Dot(dot)));
        self.graph.remove_node(dot.index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg = SegIndex::new(self.graph.add_node(Weight::Seg(weight)));
        self.graph.add_edge(seg.index, from.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        let index = TaggedIndex::Seg(seg);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
        seg

    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(TaggedIndex::Seg(seg)), TaggedIndex::Seg(seg)));
        self.graph.remove_node(seg.index);
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, around: TaggedIndex, weight: BendWeight) -> BendIndex {
        match around {
            TaggedIndex::Dot(core) =>
                self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) =>
                self.add_noncore_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
        }
    }

    pub fn add_core_bend(&mut self, from: DotIndex, to: DotIndex, core: DotIndex, weight: BendWeight) -> BendIndex {
        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));
        self.graph.add_edge(bend.index, from.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        let index = TaggedIndex::Bend(bend);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
        bend
    }

    pub fn add_noncore_bend(&mut self, from: DotIndex, to: DotIndex, inner: BendIndex, weight: BendWeight) -> BendIndex {
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
        self.rtree.remove(&RTreeWrapper::new(self.primitive(TaggedIndex::Bend(bend)), TaggedIndex::Bend(bend)));
        self.graph.remove_node(bend.index);
    }

    pub fn primitives(&self) -> Box<dyn Iterator<Item=Primitive> + '_> {
        Box::new(self.rtree.iter().map(|wrapper| self.primitive(wrapper.data)))
    }

    pub fn primitive(&self, index: TaggedIndex) -> Primitive {
        Primitive {
            weight: self.weight(index),
            dot_neighbor_weights:
                self.ends(index)
                    .into_iter()
                    .map(|index| self.dot_weight(index))
                    .collect(),
            core_pos: match index {
                TaggedIndex::Bend(bend) => {
                    Some(self.dot_weight(self.core(bend)).circle.pos)
                },
                _ => None,
            },
        }
    }

    pub fn around(&self, bend: BendIndex) -> TaggedIndex {
        if let Some(inner) = self.inner(bend) {
            TaggedIndex::Bend(inner)
        } else {
            TaggedIndex::Dot(self.core(bend))
        }
    }

    pub fn core(&self, bend: BendIndex) -> DotIndex {
        self.graph.neighbors(bend.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(bend.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .next()
            .unwrap()
    }

    pub fn inner(&self, bend: BendIndex) -> Option<BendIndex> {
        self.graph.neighbors_directed(bend.index, Incoming)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(*ni, bend.index).unwrap()).unwrap().is_outer())
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn outer(&self, bend: BendIndex) -> Option<BendIndex> {
        self.graph.neighbors_directed(bend.index, Outgoing)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(bend.index, *ni).unwrap()).unwrap().is_outer())
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn ends(&self, index: TaggedIndex) -> Vec<DotIndex> {
        match index {
            TaggedIndex::Dot(DotIndex {index: node, ..})
            | TaggedIndex::Seg(SegIndex {index: node, ..})
            | TaggedIndex::Bend(BendIndex {index: node, ..}) => {
                self.graph.neighbors(node)
                    .filter(|ni| self.graph.edge_weight(self.graph.find_edge(node, *ni).unwrap()).unwrap().is_end())
                    .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
                    .map(|ni| DotIndex::new(ni))
                    .collect()
            }
        }
    }

    pub fn dot_weight(&self, dot: DotIndex) -> DotWeight {
        *self.weight(dot.tag()).as_dot().unwrap()
    }

    pub fn seg_weight(&self, seg: SegIndex) -> SegWeight {
        *self.weight(seg.tag()).as_seg().unwrap()
    }

    pub fn bend_weight(&self, bend: BendIndex) -> BendWeight {
        *self.weight(bend.tag()).as_bend().unwrap()
    }

    pub fn weight(&self, index: TaggedIndex) -> Weight {
        match index {
            TaggedIndex::Dot(DotIndex {index: node, ..})
            | TaggedIndex::Seg(SegIndex {index: node, ..})
            | TaggedIndex::Bend(BendIndex {index: node, ..}) =>
                *self.graph.node_weight(node).unwrap(),
        }
    }
}
