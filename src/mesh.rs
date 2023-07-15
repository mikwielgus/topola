use std::marker::PhantomData;
use enum_as_inner::EnumAsInner;
use petgraph::stable_graph::{StableUnGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use rstar::{RTree, RTreeObject, AABB};
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::weight::{Weight, DotWeight, SegWeight, BendWeight, EndRefWeight, AroundRefWeight};


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

pub type SegIndex = Index<EdgeIndex<usize>, SegWeight>;

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

pub type EndRefIndex = Index<EdgeIndex<usize>, EndRefWeight>;

impl Tag for EndRefIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::EndRef(*self)
    }
}

pub type AroundRefIndex = Index<EdgeIndex<usize>, AroundRefWeight>;

impl Tag for AroundRefIndex {
    fn tag(&self) -> TaggedIndex {
        TaggedIndex::AroundRef(*self)
    }
}

#[derive(Debug, EnumAsInner, Copy, Clone, PartialEq)]
pub enum TaggedIndex {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
    EndRef(EndRefIndex),
    AroundRef(AroundRefIndex),
}

pub type RTreeWrapper = GeomWithData<Primitive, TaggedIndex>;

pub struct Mesh {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableUnGraph<Weight, Weight, usize>,
}

impl Mesh {
    pub fn new() -> Self {
        return Mesh {
            rtree: RTree::new(),
            graph: StableUnGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot_index = DotIndex::new(self.graph.add_node(Weight::Dot(weight)));
        let index = TaggedIndex::Dot(dot_index);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
        dot_index
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(TaggedIndex::Dot(dot)), TaggedIndex::Dot(dot)));
        self.graph.remove_node(dot.index);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg_index = SegIndex::new(self.graph.add_edge(from.index, to.index, Weight::Seg(weight)));
        let index = TaggedIndex::Seg(seg_index);
        self.rtree.insert(RTreeWrapper::new(self.primitive(index), index));
        seg_index
    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&RTreeWrapper::new(self.primitive(TaggedIndex::Seg(seg)), TaggedIndex::Seg(seg)));
        self.graph.remove_edge(seg.index);
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, around: TaggedIndex, weight: BendWeight) -> BendIndex {
        let bend = BendIndex::new(self.graph.add_node(Weight::Bend(weight)));
        self.graph.add_edge(from.index, bend.index, Weight::EndRef(EndRefWeight {}));
        self.graph.add_edge(bend.index, to.index, Weight::EndRef(EndRefWeight {}));

        match around {
            TaggedIndex::Dot(DotIndex {index: around_index, ..}) => {
                self.graph.add_edge(bend.index, around_index, Weight::AroundRef(AroundRefWeight {}));
            },
            TaggedIndex::Bend(BendIndex {index: around_index, ..}) => {
                self.graph.add_edge(bend.index, around_index, Weight::AroundRef(AroundRefWeight {}));
            },
            TaggedIndex::Seg(..)
            | TaggedIndex::EndRef(..)
            | TaggedIndex::AroundRef(..) =>
                unreachable!(),
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
                    .map(|index| *self.weight(index).as_dot().unwrap())
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

            if self.graph.edge_weight(edge).unwrap().is_around_ref() {
                return match self.graph.node_weight(neighbor).unwrap() {
                    Weight::Dot(dot) => DotIndex::new(neighbor).tag(),
                    Weight::Bend(bend) => BendIndex::new(neighbor).tag(),
                    Weight::Seg(..)
                    | Weight::EndRef(..)
                    | Weight::AroundRef(..) =>
                        unreachable!(),
                }
            }
        }
        unreachable!();
    }

    pub fn ends(&self, index: TaggedIndex) -> Vec<TaggedIndex> {
        match index {
            TaggedIndex::Dot(DotIndex {index: node, ..})
            | TaggedIndex::Bend(BendIndex {index: node, ..}) =>
                self.graph.neighbors(node)
                    .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
                    .filter(|ni| self.graph.edge_weight(self.graph.find_edge(node, *ni).unwrap()).unwrap().is_end_ref())
                    .map(|ni| TaggedIndex::Dot(Index::new(ni)))
                    .collect(),
            TaggedIndex::Seg(SegIndex {index: edge, ..}) => {
                let endpoints = self.graph.edge_endpoints(edge).unwrap();
                vec![TaggedIndex::Dot(DotIndex::new(endpoints.0)),
                     TaggedIndex::Dot(DotIndex::new(endpoints.1))]
            },
            TaggedIndex::EndRef(..)
            | TaggedIndex::AroundRef(..) =>
                unreachable!(),
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
            | TaggedIndex::Bend(BendIndex {index: node, ..}) =>
                *self.graph.node_weight(node).unwrap(),
            TaggedIndex::Seg(SegIndex {index: edge, ..}) =>
                *self.graph.edge_weight(edge).unwrap(),
            TaggedIndex::EndRef(..)
            | TaggedIndex::AroundRef(..) => 
                unreachable!(),
        }
    }
}
