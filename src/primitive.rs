use petgraph::Direction::{Outgoing, Incoming};
use petgraph::stable_graph::StableDiGraph;

use crate::{mesh::{DotIndex, SegIndex, BendIndex, TaggedIndex, Mesh, Tag, Index}, weight::{DotWeight, SegWeight, BendWeight, TaggedWeight, Label}};
use crate::shape::Shape;

pub struct Primitive<'a, Weight> {
    index: Index<Weight>,
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
}

impl<'a, Weight> Primitive<'a, Weight> {
    pub fn new(index: Index<Weight>, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Primitive<Weight> {
        Primitive::<Weight> {index, graph}
    }

    pub fn tagged_weight(&self) -> TaggedWeight {
        *self.graph.node_weight(self.index.index).unwrap()
    }

    fn primitive<W>(&self, index: Index<W>) -> Primitive<W> {
        Primitive::new(index, &self.graph)
    }
}

type Dot<'a> = Primitive<'a, DotWeight>;
type Seg<'a> = Primitive<'a, SegWeight>;
type Bend<'a> = Primitive<'a, BendWeight>;

impl<'a> Dot<'a> {
    pub fn shape(&self) -> Shape {
        Shape {
            weight: self.tagged_weight(),
            dot_neighbor_weights: vec![],
            core_pos: None,
        }
    }

    pub fn weight(&self) -> DotWeight {
        *self.tagged_weight().as_dot().unwrap()
    }
}

impl<'a> Seg<'a> {
    pub fn shape(&self) -> Shape {
        Shape {
            weight: self.tagged_weight(),
            dot_neighbor_weights:
                self.ends()
                    .into_iter()
                    .map(|index| self.primitive(index).weight())
                    .collect(),
            core_pos: None,
        }
    }

    pub fn ends(&self) -> Vec<DotIndex> {
        self.graph.neighbors(self.index.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_end())
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
            .map(|ni| DotIndex::new(ni))
            .collect()
    }

    pub fn weight(&self) -> SegWeight {
        *self.tagged_weight().as_seg().unwrap()
    }
}

impl<'a> Bend<'a> {
    pub fn shape(&self) -> Shape {
        Shape {
            weight: self.tagged_weight(),
            dot_neighbor_weights:
                self.ends()
                    .into_iter()
                    .map(|index| self.primitive(index).weight())
                    .collect(),
            core_pos: Some(self.primitive(self.core()).weight().circle.pos),
        }
    }

    pub fn ends(&self) -> Vec<DotIndex> {
        self.graph.neighbors(self.index.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_end())
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
            .map(|ni| DotIndex::new(ni))
            .collect()
    }

    pub fn around(&self) -> TaggedIndex {
        if let Some(inner) = self.inner() {
            TaggedIndex::Bend(inner)
        } else {
            TaggedIndex::Dot(self.core())
        }
    }

    pub fn core(&self) -> DotIndex {
        self.graph.neighbors(self.index.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .next()
            .unwrap()
    }

    pub fn inner(&self) -> Option<BendIndex> {
        self.graph.neighbors_directed(self.index.index, Incoming)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap()).unwrap().is_outer())
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<BendIndex> {
        self.graph.neighbors_directed(self.index.index, Outgoing)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_outer())
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn weight(&self) -> BendWeight {
        *self.tagged_weight().as_bend().unwrap()
    }
}
