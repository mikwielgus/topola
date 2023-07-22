use std::mem::swap;

use petgraph::Direction::{Outgoing, Incoming};
use petgraph::stable_graph::StableDiGraph;

use crate::graph::{Set, DotIndex, SegIndex, BendIndex, TaggedIndex, Tag, Index, DotWeight, SegWeight, BendWeight, TaggedWeight, Label};
use crate::shape::Shape;

pub struct Primitive<'a, Weight> {
    index: Index<Weight>,
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
}

impl<'a, Weight> Primitive<'a, Weight> {
    pub fn new(index: Index<Weight>, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Primitive<Weight> {
        Primitive::<Weight> {index, graph}
    }

    pub fn shape(&self) -> Shape {
        let ends = self.ends();
        match self.tagged_weight() {
            TaggedWeight::Dot(dot) => Shape {
                width: dot.circle.r * 2.0,
                from: dot.circle.pos,
                to: dot.circle.pos,
                center: None,
            },
            TaggedWeight::Seg(seg) => {
                Shape {
                    width: seg.width,
                    from: self.primitive(ends[0]).weight().circle.pos,
                    to: self.primitive(ends[1]).weight().circle.pos,
                    center: None,
                }
            }
            TaggedWeight::Bend(bend) => {
                let mut shape = Shape {
                    width: self.primitive(ends[0]).weight().circle.r * 2.0,
                    from: self.primitive(ends[0]).weight().circle.pos,
                    to: self.primitive(ends[1]).weight().circle.pos,
                    center: Some(self.primitive(self.core().unwrap()).weight().circle.pos),
                };

                if bend.cw {
                    swap(&mut shape.from, &mut shape.to);
                }
                shape
            }
        }
    }

    pub fn next(&self) -> Option<TaggedIndex> {
        self.graph.neighbors_directed(self.index.index, Outgoing)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap()).unwrap().is_end())
            .map(|ni| Index::<Label>::new(ni).retag(*self.graph.node_weight(ni).unwrap()))
            .next()
    }

    pub fn prev(&self) -> Option<TaggedIndex> {
        self.graph.neighbors_directed(self.index.index, Incoming)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_end())
            .map(|ni| Index::<Label>::new(ni).retag(*self.graph.node_weight(ni).unwrap()))
            .next()
    }

    pub fn ends(&self) -> Vec<DotIndex> {
        self.graph.neighbors_undirected(self.index.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge_undirected(self.index.index, *ni).unwrap().0).unwrap().is_end())
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
            .map(|ni| DotIndex::new(ni))
            .collect()
    }

    pub fn core(&self) -> Option<DotIndex> {
        self.graph.neighbors(self.index.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .next()
    }

    pub fn tagged_index(&self) -> TaggedIndex {
        self.index.retag(*self.graph.node_weight(self.index.index).unwrap())
    }

    pub fn tagged_weight(&self) -> TaggedWeight {
        *self.graph.node_weight(self.index.index).unwrap()
    }

    fn primitive<W>(&self, index: Index<W>) -> Primitive<W> {
        Primitive::new(index, &self.graph)
    }
}

impl<'a, Weight> Set for Primitive<'a, Weight> {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![self.tagged_index()]
    }

    fn closure(&self) -> Vec<TaggedIndex> {
        let ends: Vec<TaggedIndex> = self.ends()
            .into_iter()
            .map(|end| TaggedIndex::Dot(end))
            .collect();
        [[self.tagged_index()].as_slice(), ends.as_slice()].concat()
    }

    fn boundary(&self) -> Vec<DotIndex> {
        self.ends()
    }
}

type Dot<'a> = Primitive<'a, DotWeight>;
type Seg<'a> = Primitive<'a, SegWeight>;
type Bend<'a> = Primitive<'a, BendWeight>;

impl<'a> Dot<'a> {
    pub fn weight(&self) -> DotWeight {
        *self.tagged_weight().as_dot().unwrap()
    }
}

impl<'a> Seg<'a> {
    pub fn weight(&self) -> SegWeight {
        *self.tagged_weight().as_seg().unwrap()
    }
}

impl<'a> Bend<'a> {
    pub fn around(&self) -> TaggedIndex {
        if let Some(inner) = self.inner() {
            TaggedIndex::Bend(inner)
        } else {
            TaggedIndex::Dot(self.core().unwrap())
        }
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
