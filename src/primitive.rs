use std::mem::swap;

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction::{Incoming, Outgoing};

use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, Ends, GenericIndex, GetNet, GetNodeIndex, Index,
    Interior, Label, MakePrimitive, Retag, SegWeight, Weight,
};
use crate::math::{self, Circle};
use crate::shape::{BendShape, DotShape, SegShape, Shape, ShapeTrait};

#[enum_dispatch]
pub trait GetGraph {
    fn graph(&self) -> &StableDiGraph<Weight, Label, usize>;
}

#[enum_dispatch]
pub trait GetConnectable: GetNet + GetGraph {
    fn connectable(&self, index: Index) -> bool {
        let this = self.net();
        let other = index.primitive(self.graph()).net();

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
}

#[enum_dispatch]
pub trait TaggedPrevTaggedNext {
    fn tagged_prev(&self) -> Option<Index>;
    fn tagged_next(&self) -> Option<Index>;
}

#[enum_dispatch]
pub trait GetWeight<W> {
    fn weight(&self) -> W;
}

#[enum_dispatch]
pub trait MakeShape {
    fn shape(&self) -> Shape;
}

#[enum_dispatch(GetNet, GetGraph, GetConnectable, TaggedPrevTaggedNext, MakeShape)]
pub enum Primitive<'a> {
    Dot(Dot<'a>),
    Seg(Seg<'a>),
    Bend(Bend<'a>),
}

#[derive(Debug)]
pub struct GenericPrimitive<'a, W> {
    pub index: GenericIndex<W>,
    graph: &'a StableDiGraph<Weight, Label, usize>,
}

impl<'a, W> GenericPrimitive<'a, W> {
    pub fn new(index: GenericIndex<W>, graph: &'a StableDiGraph<Weight, Label, usize>) -> Self {
        Self { index, graph }
    }

    pub fn neighbors(&self) -> impl Iterator<Item = Index> + '_ {
        self.graph
            .neighbors_undirected(self.index.node_index())
            .map(|index| self.graph.node_weight(index).unwrap().retag(index))
    }

    pub fn prev_bend(&self) -> Option<BendIndex> {
        let mut prev_index = self.index.node_index();

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
            let _weight = *self.graph.node_weight(index).unwrap();

            if let Some(Weight::Bend(..)) = self.graph.node_weight(index) {
                return Some(BendIndex::new(index));
            }

            prev_index = index;
        }

        None
    }

    fn prev_node(&self) -> Option<NodeIndex<usize>> {
        self.graph
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
    }

    pub fn next_bend(&self) -> Option<BendIndex> {
        let mut prev_index = self.index.node_index();

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
            let _weight = *self.graph.node_weight(index).unwrap();

            if let Some(Weight::Bend(..)) = self.graph.node_weight(index) {
                return Some(BendIndex::new(index));
            }

            prev_index = index;
        }

        None
    }

    fn next_node(&self) -> Option<NodeIndex<usize>> {
        self.graph
            .neighbors_directed(self.index.node_index(), Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_end()
            })
            .next()
    }

    pub fn core(&self) -> Option<DotIndex> {
        self.graph
            .neighbors(self.index.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| DotIndex::new(ni))
            .next()
    }

    /*pub fn connectable<WW>(&self, index: GenericIndex<WW>) -> bool {
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

    fn net<WW>(&self, index: &GenericIndex<WW>) -> i64 {
        match self.graph.node_weight(index.node_index()).unwrap() {
            Weight::Dot(dot) => dot.net,
            Weight::Seg(seg) => seg.net,
            Weight::Bend(bend) => bend.net,
        }
    }*/

    pub fn tagged_index(&self) -> Index {
        self.graph
            .node_weight(self.index.node_index())
            .unwrap()
            .retag(self.index.node_index())
    }

    pub fn tagged_weight(&self) -> Weight {
        *self.graph.node_weight(self.index.node_index()).unwrap()
    }

    fn primitive<WW>(&self, index: GenericIndex<WW>) -> GenericPrimitive<WW> {
        GenericPrimitive::new(index, &self.graph)
    }
}

impl<'a, W> Interior<Index> for GenericPrimitive<'a, W> {
    fn interior(&self) -> Vec<Index> {
        vec![self.tagged_index()]
    }
}

impl<'a, W> Ends<DotIndex, DotIndex> for GenericPrimitive<'a, W> {
    fn ends(&self) -> (DotIndex, DotIndex) {
        let v = self
            .graph
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(
                        self.graph
                            .find_edge_undirected(self.index.node_index(), *ni)
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

impl<'a, W> GetGraph for GenericPrimitive<'a, W> {
    fn graph(&self) -> &StableDiGraph<Weight, Label, usize> {
        self.graph
    }
}

impl<'a, W: GetNet> GetConnectable for GenericPrimitive<'a, W> where
    GenericPrimitive<'a, W>: GetWeight<W>
{
}

impl<'a, W: GetNet> GetNet for GenericPrimitive<'a, W>
where
    GenericPrimitive<'a, W>: GetWeight<W>,
{
    fn net(&self) -> i64 {
        self.weight().net()
    }
}

impl<'a, W> TaggedPrevTaggedNext for GenericPrimitive<'a, W> {
    fn tagged_prev(&self) -> Option<Index> {
        self.prev_node()
            .map(|ni| self.graph.node_weight(ni).unwrap().retag(ni))
    }

    fn tagged_next(&self) -> Option<Index> {
        self.next_node()
            .map(|ni| self.graph.node_weight(ni).unwrap().retag(ni))
    }
}

pub type Dot<'a> = GenericPrimitive<'a, DotWeight>;

impl<'a> Dot<'a> {
    pub fn bend(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(
                        self.graph
                            .find_edge_undirected(self.index.node_index(), *ni)
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
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| BendIndex::new(ni))
            .filter(|bend| self.primitive(*bend).inner().is_none())
            .next()
    }
}

impl<'a> GetWeight<DotWeight> for Dot<'a> {
    fn weight(&self) -> DotWeight {
        self.tagged_weight().into_dot().unwrap()
    }
}

impl<'a> MakeShape for Dot<'a> {
    fn shape(&self) -> Shape {
        Shape::Dot(DotShape {
            c: self.weight().circle,
        })
    }
}

pub type Seg<'a> = GenericPrimitive<'a, SegWeight>;

impl<'a> Seg<'a> {
    pub fn next(&self) -> Option<DotIndex> {
        self.next_node().map(|ni| DotIndex::new(ni))
    }

    pub fn prev(&self) -> Option<DotIndex> {
        self.prev_node().map(|ni| DotIndex::new(ni))
    }
}

impl<'a> GetWeight<SegWeight> for Seg<'a> {
    fn weight(&self) -> SegWeight {
        self.tagged_weight().into_seg().unwrap()
    }
}

impl<'a> MakeShape for Seg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.weight().width,
        })
    }
}

pub type Bend<'a> = GenericPrimitive<'a, BendWeight>;

impl<'a> Bend<'a> {
    pub fn around(&self) -> Index {
        if let Some(inner) = self.inner() {
            Index::Bend(inner)
        } else {
            Index::Dot(self.core().unwrap())
        }
    }

    pub fn inner(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| BendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<BendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.node_index(), *ni).unwrap())
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

    fn inner_radius(&self) -> f64 {
        let mut r = 0.0;
        let mut layer = BendIndex::new(self.index.node_index());

        while let Some(inner) = self.primitive(layer).inner() {
            r += self.primitive(inner).shape().width();
            layer = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(BendIndex::new(self.index.node_index()))
                    .core()
                    .unwrap(),
            )
            .weight()
            .circle;

        core_circle.r + r + 3.0
    }

    pub fn cross_product(&self) -> f64 {
        let center = self.primitive(self.core().unwrap()).weight().circle.pos;
        let ends = self.ends();
        let end1 = self.primitive(ends.0).weight().circle.pos;
        let end2 = self.primitive(ends.1).weight().circle.pos;
        math::cross_product(end1 - center, end2 - center)
    }
}

impl<'a> GetWeight<BendWeight> for Bend<'a> {
    fn weight(&self) -> BendWeight {
        self.tagged_weight().into_bend().unwrap()
    }
}

impl<'a> MakeShape for Bend<'a> {
    fn shape(&self) -> Shape {
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

        if self.weight().cw {
            swap(&mut bend_shape.from, &mut bend_shape.to);
        }
        Shape::Bend(bend_shape)
    }
}
