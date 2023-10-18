use std::mem::{self, swap};

use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction::{Incoming, Outgoing};

use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, Ends, Index, Interior, Label, SegWeight,
    TaggedIndex, TaggedWeight,
};
use crate::math::{self, Circle};
use crate::shape::{BendShape, DotShape, SegShape, Shape, ShapeTrait};

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
                    from: self.primitive(ends.0).weight().circle.pos,
                    to: self.primitive(ends.1).weight().circle.pos,
                    width: seg.width,
                })
            }
            TaggedWeight::Bend(bend) => {
                let ends = self.ends();

                let mut bend_shape = BendShape {
                    from: self.primitive(ends.0).weight().circle.pos,
                    to: self.primitive(ends.1).weight().circle.pos,
                    c: Circle {
                        pos: self.primitive(self.core().unwrap()).weight().circle.pos,
                        r: self.inner_radius(),
                    },
                    width: self.primitive(ends.0).weight().circle.r * 2.0,
                };

                if bend.cw {
                    swap(&mut bend_shape.from, &mut bend_shape.to);
                }
                Shape::Bend(bend_shape)
            }
        }
    }

    fn inner_radius(&self) -> f64 {
        let mut r = 0.0;
        let mut layer = BendIndex::new(self.index.index);

        while let Some(inner) = self.primitive(layer).inner() {
            r += self.primitive(inner).shape().width();
            layer = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(BendIndex::new(self.index.index))
                    .core()
                    .unwrap(),
            )
            .weight()
            .circle;

        core_circle.r + r + 3.0
    }

    pub fn neighbors(&self) -> impl Iterator<Item = TaggedIndex> + '_ {
        self.graph
            .neighbors_undirected(self.index.index)
            .map(|index| Index::<Label>::new(index).retag(self.graph.node_weight(index).unwrap()))
    }

    pub fn prev_bend(&self) -> Option<BendIndex> {
        let mut prev_index = self.index.index;

        while let Some(index) = self
            .graph
            .neighbors_directed(prev_index, Incoming)
            // Ensure subsequent unwrap doesn't panic.
            .filter(|ni| self.graph.find_edge(*ni, prev_index).is_some())
            // Filter out non-End edges.
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, prev_index).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
        {
            let weight = *self.graph.node_weight(index).unwrap();

            if let Some(TaggedWeight::Bend(..)) = self.graph.node_weight(index) {
                return Some(BendIndex::new(index));
            }

            prev_index = index;
        }

        None
    }

    pub fn tagged_prev(&self) -> Option<TaggedIndex> {
        self.prev_node()
            .map(|ni| Index::<Label>::new(ni).retag(self.graph.node_weight(ni).unwrap()))
    }

    fn prev_node(&self) -> Option<NodeIndex<usize>> {
        self.graph
            .neighbors_directed(self.index.index, Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.index).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
    }

    pub fn next_bend(&self) -> Option<BendIndex> {
        let mut prev_index = self.index.index;

        while let Some(index) = self
            .graph
            .neighbors_directed(prev_index, Outgoing)
            // Ensure subsequent unwrap doesn't panic.
            .filter(|ni| self.graph.find_edge(prev_index, *ni).is_some())
            // Filter out non-End edges.
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(prev_index, *ni).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
        {
            let weight = *self.graph.node_weight(index).unwrap();

            if let Some(TaggedWeight::Bend(..)) = self.graph.node_weight(index) {
                return Some(BendIndex::new(index));
            }

            prev_index = index;
        }

        None
    }

    pub fn tagged_next(&self) -> Option<TaggedIndex> {
        self.next_node()
            .map(|ni| Index::<Label>::new(ni).retag(self.graph.node_weight(ni).unwrap()))
    }

    fn next_node(&self) -> Option<NodeIndex<usize>> {
        self.graph
            .neighbors_directed(self.index.index, Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.index, *ni).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
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

    pub fn connectable<W>(&self, index: Index<W>) -> bool {
        let this = self.net(&self.index);
        let other = self.net(&index);

        if this == other {
            true
        } else if this == -1 || other == -1 {
            true
        } else if this == -2 || other == -2 {
            false
        } else {
            this == other
        }
    }

    fn net<W>(&self, index: &Index<W>) -> i64 {
        match self.graph.node_weight(index.index).unwrap() {
            TaggedWeight::Dot(dot) => dot.net,
            TaggedWeight::Seg(seg) => seg.net,
            TaggedWeight::Bend(bend) => bend.net,
        }
    }

    pub fn tagged_index(&self) -> TaggedIndex {
        self.index
            .retag(self.graph.node_weight(self.index.index).unwrap())
    }

    pub fn tagged_weight(&self) -> TaggedWeight {
        *self.graph.node_weight(self.index.index).unwrap()
    }

    fn primitive<W>(&self, index: Index<W>) -> Primitive<W> {
        Primitive::new(index, &self.graph)
    }
}

impl<'a, Weight> Interior<TaggedIndex> for Primitive<'a, Weight> {
    fn interior(&self) -> Vec<TaggedIndex> {
        vec![self.tagged_index()]
    }
}

impl<'a, Weight> Ends<DotIndex, DotIndex> for Primitive<'a, Weight> {
    fn ends(&self) -> (DotIndex, DotIndex) {
        let v = self
            .graph
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
            .collect::<Vec<_>>();
        (v[0], v[1])
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
        self.tagged_weight().into_dot().unwrap()
    }
}

impl<'a> Seg<'a> {
    pub fn next(&self) -> Option<DotIndex> {
        self.next_node().map(|ni| DotIndex::new(ni))
    }

    pub fn prev(&self) -> Option<DotIndex> {
        self.prev_node().map(|ni| DotIndex::new(ni))
    }

    pub fn weight(&self) -> SegWeight {
        self.tagged_weight().into_seg().unwrap()
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

    pub fn next(&self) -> Option<DotIndex> {
        self.next_node().map(|ni| DotIndex::new(ni))
    }

    pub fn prev(&self) -> Option<DotIndex> {
        self.prev_node().map(|ni| DotIndex::new(ni))
    }

    pub fn weight(&self) -> BendWeight {
        self.tagged_weight().into_bend().unwrap()
    }

    pub fn cross_product(&self) -> f64 {
        let center = self.primitive(self.core().unwrap()).weight().circle.pos;
        let ends = self.ends();
        let end1 = self.primitive(ends.0).weight().circle.pos;
        let end2 = self.primitive(ends.1).weight().circle.pos;
        math::cross_product(end1 - center, end2 - center)
    }
}
