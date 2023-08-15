use std::mem::{self, swap};

use petgraph::stable_graph::StableDiGraph;
use petgraph::Direction::{Incoming, Outgoing};

use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, Index, Label, Path, SegIndex, SegWeight, Tag,
    TaggedIndex, TaggedWeight,
};
use crate::math;
use crate::shape::{BendShape, DotShape, SegShape, Shape};

#[derive(Debug)]
pub struct Primitive<'a, Weight> {
    pub index: Index<Weight>,
    graph: &'a StableDiGraph<TaggedWeight, Label, usize>,
}

impl<'a, Weight> Primitive<'a, Weight> {
    pub fn new(index: Index<Weight>, graph: &'a StableDiGraph<TaggedWeight, Label, usize>) -> Self {
        Self { index, graph }
    }

    pub fn shape(&self) -> Shape {
        match self.tagged_weight() {
            TaggedWeight::Dot(dot) => Shape::Dot(DotShape { c: dot.circle }),
            TaggedWeight::Seg(seg) => {
                let ends = self.ends();
                Shape::Seg(SegShape {
                    from: self.primitive(ends[0]).weight().circle.pos,
                    to: self.primitive(ends[1]).weight().circle.pos,
                    width: seg.width,
                })
            }
            TaggedWeight::Bend(bend) => {
                let ends = self.ends();
                let mut bend_shape = BendShape {
                    from: self.primitive(ends[0]).weight().circle.pos,
                    to: self.primitive(ends[1]).weight().circle.pos,
                    center: self.primitive(self.core().unwrap()).weight().circle.pos,
                    width: self.primitive(ends[0]).weight().circle.r * 2.0,
                };

                if bend.cw {
                    swap(&mut bend_shape.from, &mut bend_shape.to);
                }
                Shape::Bend(bend_shape)
            }
        }
    }

    pub fn next(&self) -> Option<TaggedIndex> {
        self.graph
            .neighbors_directed(self.index.index, Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap())
                    .unwrap()
                    .is_end()
            })
            .map(|ni| Index::<Label>::new(ni).retag(*self.graph.node_weight(ni).unwrap()))
            .next()
    }

    pub fn next_akin(&self) -> Option<Index<Weight>> {
        let mut prev_index = self.index.index;

        while let Some(index) = self
            .graph
            .neighbors_directed(prev_index, Outgoing)
            .filter(|ni| self.graph.find_edge(prev_index, *ni).is_some())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(prev_index, *ni).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
        {
            let weight = *self.graph.node_weight(index).unwrap();

            if mem::discriminant(&self.tagged_weight()) == mem::discriminant(&weight) {
                return Some(Index::<Weight>::new(index));
            }

            prev_index = index;
        }

        None
    }

    pub fn prev(&self) -> Option<TaggedIndex> {
        self.graph
            .neighbors_directed(self.index.index, Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap())
                    .unwrap()
                    .is_end()
            })
            .map(|ni| Index::<Label>::new(ni).retag(*self.graph.node_weight(ni).unwrap()))
            .next()
    }

    pub fn prev_akin(&self) -> Option<Index<Weight>> {
        let mut prev_index = self.index.index;

        while let Some(index) = self
            .graph
            .neighbors_directed(prev_index, Incoming)
            .filter(|ni| self.graph.find_edge(*ni, prev_index).is_some())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, prev_index).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
        {
            let weight = *self.graph.node_weight(index).unwrap();

            if mem::discriminant(&self.tagged_weight()) == mem::discriminant(&weight) {
                return Some(Index::<Weight>::new(index));
            }

            prev_index = index;
        }

        None
    }

    pub fn core(&self) -> Option<DotIndex> {
        self.graph
            .neighbors(self.index.index)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| DotIndex::new(ni))
            .next()
    }

    pub fn tagged_index(&self) -> TaggedIndex {
        self.index
            .retag(*self.graph.node_weight(self.index.index).unwrap())
    }

    pub fn tagged_weight(&self) -> TaggedWeight {
        *self.graph.node_weight(self.index.index).unwrap()
    }

    fn primitive<W>(&self, index: Index<W>) -> Primitive<W> {
        Primitive::new(index, &self.graph)
    }
}

impl<'a, Weight> Path for Primitive<'a, Weight> {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![self.tagged_index()]
    }

    fn closure(&self) -> Vec<TaggedIndex> {
        let ends: Vec<TaggedIndex> = self
            .ends()
            .into_iter()
            .map(|end| TaggedIndex::Dot(end))
            .collect();
        [[self.tagged_index()].as_slice(), ends.as_slice()].concat()
    }

    fn ends(&self) -> [DotIndex; 2] {
        self.graph
            .neighbors_undirected(self.index.index)
            .filter(|ni| {
                self.graph
                    .edge_weight(
                        self.graph
                            .find_edge_undirected(self.index.index, *ni)
                            .unwrap()
                            .0,
                    )
                    .unwrap()
                    .is_end()
            })
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_dot())
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .try_into()
            .unwrap()
    }
}

pub type Dot<'a> = Primitive<'a, DotWeight>;
pub type Seg<'a> = Primitive<'a, SegWeight>;
pub type Bend<'a> = Primitive<'a, BendWeight>;

impl<'a> Dot<'a> {
    pub fn bend(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_undirected(self.index.index)
            .filter(|ni| {
                self.graph
                    .edge_weight(
                        self.graph
                            .find_edge_undirected(self.index.index, *ni)
                            .unwrap()
                            .0,
                    )
                    .unwrap()
                    .is_end()
            })
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_bend())
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_directed(self.index.index, Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| BendIndex::new(ni))
            .filter(|bend| self.primitive(*bend).inner().is_none())
            .next()
    }

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
        self.graph
            .neighbors_directed(self.index.index, Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_directed(self.index.index, Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn weight(&self) -> BendWeight {
        *self.tagged_weight().as_bend().unwrap()
    }

    pub fn cross_product(&self) -> f64 {
        let center = self.primitive(self.core().unwrap()).weight().circle.pos;
        let ends = self.ends();
        let end1 = self.primitive(ends[0]).weight().circle.pos;
        let end2 = self.primitive(ends[1]).weight().circle.pos;
        math::cross_product(end1 - center, end2 - center)
    }
}
