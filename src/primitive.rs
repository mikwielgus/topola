use std::mem::swap;

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction::{Incoming, Outgoing};

use crate::graph::{
    Ends, FixedBendIndex, FixedBendWeight, FixedDotIndex, FixedDotWeight, FixedSegIndex,
    FixedSegWeight, FullyLooseSegWeight, GenericIndex, GetNet, GetNodeIndex, HalfLooseSegWeight,
    Index, Interior, Label, LooseBendIndex, LooseBendWeight, LooseDotWeight, MakePrimitive, Retag,
    Weight,
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

        (this == other) || this == -1 || other == -1
    }
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
    FixedDot(FixedDot<'a>),
    LooseDot(LooseDot<'a>),
    FixedSeg(FixedSeg<'a>),
    HalfLooseSeg(HalfLooseSeg<'a>),
    FullyLooseSeg(FullyLooseSeg<'a>),
    FixedBend(FixedBend<'a>),
    LooseBend(LooseBend<'a>),
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

    pub fn core(&self) -> Option<FixedDotIndex> {
        self.graph
            .neighbors(self.index.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| FixedDotIndex::new(ni))
            .next()
    }

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

impl<'a, W> Ends<FixedDotIndex, FixedDotIndex> for GenericPrimitive<'a, W> {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
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
                    .is_adjacent()
            })
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_fixed_dot())
            .map(|ni| FixedDotIndex::new(ni))
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

pub type FixedDot<'a> = GenericPrimitive<'a, FixedDotWeight>;

impl<'a> FixedDot<'a> {
    pub fn seg(&self) -> Option<FixedSegIndex> {
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
                    .is_adjacent()
            })
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_fixed_seg())
            .map(|ni| FixedSegIndex::new(ni))
            .next()
    }

    pub fn bend(&self) -> Option<FixedBendIndex> {
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
                    .is_adjacent()
            })
            .filter(|ni| self.graph.node_weight(*ni).unwrap().is_fixed_bend())
            .map(|ni| FixedBendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<FixedBendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| FixedBendIndex::new(ni))
            .filter(|bend| self.primitive(*bend).inner().is_none())
            .next()
    }
}

impl<'a> GetWeight<FixedDotWeight> for FixedDot<'a> {
    fn weight(&self) -> FixedDotWeight {
        self.tagged_weight().into_fixed_dot().unwrap()
    }
}

impl<'a> MakeShape for FixedDot<'a> {
    fn shape(&self) -> Shape {
        Shape::Dot(DotShape {
            c: self.weight().circle,
        })
    }
}

pub type LooseDot<'a> = GenericPrimitive<'a, LooseDotWeight>;

impl<'a> GetWeight<LooseDotWeight> for LooseDot<'a> {
    fn weight(&self) -> LooseDotWeight {
        self.tagged_weight().into_loose_dot().unwrap()
    }
}

impl<'a> MakeShape for LooseDot<'a> {
    fn shape(&self) -> Shape {
        Shape::Dot(DotShape {
            c: self.weight().circle,
        })
    }
}

pub type FixedSeg<'a> = GenericPrimitive<'a, FixedSegWeight>;

impl<'a> FixedSeg<'a> {
    pub fn other_end(&self, dot: FixedDotIndex) -> FixedDotIndex {
        let ends = self.ends();
        [ends.0, ends.1]
            .into_iter()
            .find(|end| end.node_index() != dot.node_index())
            .unwrap()
    }
}

impl<'a> GetWeight<FixedSegWeight> for FixedSeg<'a> {
    fn weight(&self) -> FixedSegWeight {
        self.tagged_weight().into_fixed_seg().unwrap()
    }
}

impl<'a> MakeShape for FixedSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.weight().width,
        })
    }
}

pub type HalfLooseSeg<'a> = GenericPrimitive<'a, HalfLooseSegWeight>;

impl<'a> GetWeight<HalfLooseSegWeight> for HalfLooseSeg<'a> {
    fn weight(&self) -> HalfLooseSegWeight {
        self.tagged_weight().into_half_loose_seg().unwrap()
    }
}

impl<'a> MakeShape for HalfLooseSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.weight().width,
        })
    }
}

pub type FullyLooseSeg<'a> = GenericPrimitive<'a, FullyLooseSegWeight>;

impl<'a> GetWeight<FullyLooseSegWeight> for FullyLooseSeg<'a> {
    fn weight(&self) -> FullyLooseSegWeight {
        self.tagged_weight().into_fully_loose_seg().unwrap()
    }
}

impl<'a> MakeShape for FullyLooseSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.weight().width,
        })
    }
}

pub type FixedBend<'a> = GenericPrimitive<'a, FixedBendWeight>;

impl<'a> FixedBend<'a> {
    pub fn around(&self) -> Index {
        if let Some(inner) = self.inner() {
            inner.into()
        } else {
            self.core().unwrap().into()
        }
    }

    pub fn inner(&self) -> Option<FixedBendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| FixedBendIndex::new(ni))
            .next()
    }

    pub fn outer(&self) -> Option<FixedBendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Outgoing)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(self.index.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| FixedBendIndex::new(ni))
            .next()
    }

    pub fn other_end(&self, dot: FixedDotIndex) -> FixedDotIndex {
        let ends = self.ends();
        [ends.0, ends.1]
            .into_iter()
            .find(|end| end.node_index() != dot.node_index())
            .unwrap()
    }

    fn inner_radius(&self) -> f64 {
        let mut r = 0.0;
        let mut layer = FixedBendIndex::new(self.index.node_index());

        while let Some(inner) = self.primitive(layer).inner() {
            r += self.primitive(inner).shape().width();
            layer = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(FixedBendIndex::new(self.index.node_index()))
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

impl<'a> GetWeight<FixedBendWeight> for FixedBend<'a> {
    fn weight(&self) -> FixedBendWeight {
        self.tagged_weight().into_fixed_bend().unwrap()
    }
}

impl<'a> MakeShape for FixedBend<'a> {
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

pub type LooseBend<'a> = GenericPrimitive<'a, LooseBendWeight>;

impl<'a> LooseBend<'a> {
    pub fn inner(&self) -> Option<LooseBendIndex> {
        self.graph
            .neighbors_directed(self.index.node_index(), Incoming)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(*ni, self.index.node_index()).unwrap())
                    .unwrap()
                    .is_outer()
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }

    fn inner_radius(&self) -> f64 {
        let mut r = 0.0;
        let mut layer = LooseBendIndex::new(self.index.node_index());

        while let Some(inner) = self.primitive(layer).inner() {
            r += self.primitive(inner).shape().width();
            layer = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(LooseBendIndex::new(self.index.node_index()))
                    .core()
                    .unwrap(),
            )
            .weight()
            .circle;

        core_circle.r + r + 3.0
    }
}

impl<'a> GetWeight<LooseBendWeight> for LooseBend<'a> {
    fn weight(&self) -> LooseBendWeight {
        self.tagged_weight().into_loose_bend().unwrap()
    }
}

impl<'a> MakeShape for LooseBend<'a> {
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
