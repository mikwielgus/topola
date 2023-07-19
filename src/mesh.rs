use std::marker::PhantomData;
use enum_as_inner::EnumAsInner;
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
        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));
        self.graph.add_edge(bend.index, from.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);

        match around {
            TaggedIndex::Dot(DotIndex {index: around_index, ..}) => {
                self.graph.add_edge(bend.index, around_index, Label::Around);
            },
            TaggedIndex::Seg(..) => unreachable!(),
            TaggedIndex::Bend(BendIndex {index: around_index, ..}) => {
                self.graph.add_edge(bend.index, around_index, Label::Around);
            },
        }

        let index = TaggedIndex::Bend(bend);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
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
            around_weight: match index {
                TaggedIndex::Bend(bend) => Some(self.weight(self.around(bend))),
                _ => None,
            },
            focus: match index {
                TaggedIndex::Bend(bend) => {
                    let mut layer = index;
                    while let TaggedIndex::Bend(layer_bend) = layer {
                        layer = self.around(layer_bend)
                    }
                    Some(self.weight(layer).as_dot().unwrap().circle.pos)
                },
                _ => None,
            },
        }
    }

    /*pub fn focus(&self, bend: BendIndex) -> DotIndex {
        let mut layer = bend.tag();
        while let TaggedIndex::Bend(bend) = layer {
            layer = self.around(layer).unwrap();
        }

        *layer.as_dot().unwrap()
    }*/

    pub fn around(&self, bend: BendIndex) -> TaggedIndex {
        for neighbor in self.graph.neighbors(bend.index) {
            let edge = self.graph.find_edge(bend.index, neighbor).unwrap();

            if self.graph.edge_weight(edge).unwrap().is_around() {
                return match self.graph.node_weight(neighbor).unwrap() {
                    Weight::Dot(dot) => DotIndex::new(neighbor).tag(),
                    Weight::Bend(bend) => BendIndex::new(neighbor).tag(),
                    Weight::Seg(seg) => SegIndex::new(neighbor).tag(),
                }
            }
        }
        unreachable!();
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
