use enum_as_inner::EnumAsInner;
use petgraph::stable_graph::{StableUnGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use rstar::{RTree, RTreeObject, AABB};
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::weight::{Weight, DotWeight, SegWeight, BendWeight};

pub type DotIndex = NodeIndex<u32>;
pub type SegIndex = EdgeIndex<u32>;
pub type BendIndex = EdgeIndex<u32>;

#[derive(EnumAsInner, Copy, Clone, PartialEq)]
pub enum Index {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
}

pub type IndexRTreeWrapper = GeomWithData<Primitive, Index>;

pub struct Mesh {
    pub rtree: RTree<IndexRTreeWrapper>,
    pub graph: StableUnGraph<Weight, Weight, u32>,
}

impl Default for Mesh {
    fn default() -> Self {
        return Mesh::new();
    }
}

impl Mesh {
    pub fn new() -> Self {
        return Mesh {
            rtree: RTree::new(),
            graph: StableUnGraph::default(),
        }
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot_index = self.graph.add_node(Weight::Dot(weight));
        let index = Index::Dot(dot_index);
        self.rtree.insert(IndexRTreeWrapper::new(self.primitive(index), index));
        dot_index
    }

    pub fn remove_dot(&mut self, dot: DotIndex) {
        self.rtree.remove(&IndexRTreeWrapper::new(self.primitive(Index::Dot(dot)), Index::Dot(dot)));
        self.graph.remove_node(dot);
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg_index = self.graph.add_edge(from, to, Weight::Seg(weight));
        let index = Index::Seg(seg_index);
        self.rtree.insert(IndexRTreeWrapper::new(self.primitive(index), index));
        seg_index
    }

    pub fn remove_seg(&mut self, seg: SegIndex) {
        self.rtree.remove(&IndexRTreeWrapper::new(self.primitive(Index::Seg(seg)), Index::Seg(seg)));
        self.graph.remove_edge(seg);
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, weight: BendWeight) -> BendIndex {
        let bend_index = self.graph.add_edge(from, to, Weight::Bend(weight));
        let index = Index::Bend(bend_index);
        self.rtree.insert(IndexRTreeWrapper::new(self.primitive(index), index));
        bend_index
    }

    pub fn remove_bend(&mut self, bend: BendIndex) {
        self.rtree.remove(&IndexRTreeWrapper::new(self.primitive(Index::Bend(bend)), Index::Bend(bend)));
        self.graph.remove_edge(bend);
    }

    pub fn primitives(&self) -> Box<dyn Iterator<Item=Primitive> + '_> {
        Box::new(self.rtree.iter().map(|wrapper| self.primitive(wrapper.data)))
    }

    pub fn primitive(&self, index: Index) -> Primitive {
        Primitive {
            weight: self.weight(index),
            dot_neighbor_weights:
                self.dot_neighbors(index)
                    .into_iter()
                    .map(|index| *self.weight(index).as_dot().unwrap())
                    .collect(),
            around_weight: match index {
                Index::Bend(bend_index) => Some(*self.weight(Index::Dot((*self.weight(index).as_bend().unwrap()).around)).as_dot().unwrap()),
                _ => None,
            }
        }
    }

    pub fn dot_neighbors(&self, index: Index) -> Vec<Index> {
        match index {
            Index::Dot(node_index) =>
                return self.graph.neighbors(node_index).map(|ni| Index::Dot(ni)).collect(),
            Index::Seg(edge_index) | Index::Bend(edge_index) => {
                let endpoints = self.graph.edge_endpoints(edge_index).unwrap();
                return vec![Index::Dot(endpoints.0), Index::Dot(endpoints.1)]
            }
        }
    }

    pub fn bend(&self, index: NodeIndex) -> Option<BendIndex> {
        let edges: Vec<EdgeIndex<u32>> = self.graph.edges(index).map(|r| r.id()).collect();

        if edges.len() != 1 {
            return None;
        }

        return Some(edges[0]);
    }

    pub fn weight(&self, index: Index) -> Weight {
        return match index {
            Index::Dot(node_index) =>
                *self.graph.node_weight(node_index).unwrap(),
            Index::Seg(edge_index) | Index::Bend(edge_index) =>
                *self.graph.edge_weight(edge_index).unwrap(),
        }
    }
}
