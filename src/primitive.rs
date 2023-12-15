use std::mem::swap;

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction::{Incoming, Outgoing};

use crate::graph::{
    DotIndex, FixedBendIndex, FixedBendWeight, FixedDotIndex, FixedDotWeight, FixedSegWeight,
    GenericIndex, GetBand, GetEnds, GetNet, GetNodeIndex, GetWidth, Index, Interior, Label,
    LooseBendIndex, LooseBendWeight, LooseDotIndex, LooseDotWeight, LooseSegIndex, LooseSegWeight,
    MakePrimitive, Retag, Weight,
};
use crate::layout::Layout;
use crate::math::{self, Circle};
use crate::shape::{BendShape, DotShape, SegShape, Shape, ShapeTrait};
use crate::traverser::OutwardRailTraverser;

#[enum_dispatch]
pub trait GetLayout {
    fn layout(&self) -> &Layout;
}

#[enum_dispatch]
pub trait GetConnectable: GetNet + GetLayout {
    fn connectable(&self, index: Index) -> bool {
        let this = self.net();
        let other = index.primitive(self.layout()).net();

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

pub trait GetOtherEnd<F: GetNodeIndex, T: GetNodeIndex + Into<F>>: GetEnds<F, T> {
    fn other_end(&self, end: F) -> F {
        let ends = self.ends();
        if ends.0.node_index() != end.node_index() {
            ends.0
        } else {
            ends.1.into()
        }
    }
}

pub trait TraverseOutward: GetFirstRail {
    fn traverse_outward(&self) -> OutwardRailTraverser {
        OutwardRailTraverser::new(self.first_rail(), self.layout())
    }
}

pub trait GetFirstRail: GetLayout + GetNodeIndex {
    fn first_rail(&self) -> Option<LooseBendIndex> {
        self.layout()
            .graph
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|ni| {
                self.layout()
                    .graph
                    .find_edge(self.node_index(), *ni)
                    .is_some()
            })
            .filter(|ni| {
                matches!(
                    self.layout()
                        .graph
                        .edge_weight(
                            self.layout()
                                .graph
                                .find_edge(self.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    Label::Core
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }
}

pub trait GetCore: GetLayout + GetNodeIndex {
    fn core(&self) -> FixedDotIndex {
        self.layout()
            .graph
            .neighbors(self.node_index())
            .filter(|ni| {
                matches!(
                    self.layout()
                        .graph
                        .edge_weight(
                            self.layout()
                                .graph
                                .find_edge(self.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    Label::Core
                )
            })
            .map(|ni| FixedDotIndex::new(ni))
            .next()
            .unwrap()
    }
}

pub trait GetInnerOuter: GetLayout + GetNodeIndex {
    fn inner(&self) -> Option<LooseBendIndex> {
        self.layout()
            .graph
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|ni| {
                matches!(
                    self.layout()
                        .graph
                        .edge_weight(
                            self.layout()
                                .graph
                                .find_edge(*ni, self.node_index())
                                .unwrap()
                        )
                        .unwrap(),
                    Label::Outer
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.layout()
            .graph
            .neighbors_directed(self.node_index(), Outgoing)
            .filter(|ni| {
                matches!(
                    self.layout()
                        .graph
                        .edge_weight(
                            self.layout()
                                .graph
                                .find_edge(self.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    Label::Outer
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }
}

macro_rules! impl_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl<'a> GetWeight<$weight_struct> for $primitive_struct<'a> {
            fn weight(&self) -> $weight_struct {
                if let Weight::$primitive_struct(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }
    };
}

macro_rules! impl_fixed_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_struct, $weight_struct);

        impl<'a> GetNet for $primitive_struct<'a> {
            fn net(&self) -> i64 {
                self.weight().net()
            }
        }
    };
}

macro_rules! impl_loose_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_struct, $weight_struct);

        impl<'a> GetNet for $primitive_struct<'a> {
            fn net(&self) -> i64 {
                self.layout().bands[self.weight().band()].net
            }
        }
    };
}

#[enum_dispatch(GetNet, GetWidth, GetLayout, GetConnectable, MakeShape)]
pub enum Primitive<'a> {
    FixedDot(FixedDot<'a>),
    LooseDot(LooseDot<'a>),
    FixedSeg(FixedSeg<'a>),
    LooseSeg(LooseSeg<'a>),
    FixedBend(FixedBend<'a>),
    LooseBend(LooseBend<'a>),
}

#[derive(Debug)]
pub struct GenericPrimitive<'a, W> {
    pub index: GenericIndex<W>,
    layout: &'a Layout,
}

impl<'a, W> GenericPrimitive<'a, W> {
    pub fn new(index: GenericIndex<W>, layout: &'a Layout) -> Self {
        Self { index, layout }
    }

    fn tagged_weight(&self) -> Weight {
        *self
            .layout
            .graph
            .node_weight(self.index.node_index())
            .unwrap()
    }

    fn adjacents(&self) -> Vec<NodeIndex<usize>> {
        self.layout
            .graph
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .graph
                        .edge_weight(
                            self.layout
                                .graph
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    Label::Adjacent
                )
            })
            .collect()
    }

    fn primitive<WW>(&self, index: GenericIndex<WW>) -> GenericPrimitive<WW> {
        GenericPrimitive::new(index, &self.layout)
    }
}

impl<'a, W> Interior<Index> for GenericPrimitive<'a, W> {
    fn interior(&self) -> Vec<Index> {
        vec![self.tagged_weight().retag(self.index.node_index())]
    }
}

impl<'a, W> GetLayout for GenericPrimitive<'a, W> {
    fn layout(&self) -> &Layout {
        self.layout
    }
}

impl<'a, W> GetNodeIndex for GenericPrimitive<'a, W> {
    fn node_index(&self) -> NodeIndex<usize> {
        self.index.node_index()
    }
}

impl<'a, W> GetConnectable for GenericPrimitive<'a, W> where GenericPrimitive<'a, W>: GetNet {}

impl<'a, W: GetWidth> GetWidth for GenericPrimitive<'a, W>
where
    GenericPrimitive<'a, W>: GetWeight<W>,
{
    fn width(&self) -> f64 {
        self.weight().width()
    }
}

pub type FixedDot<'a> = GenericPrimitive<'a, FixedDotWeight>;
impl_fixed_primitive!(FixedDot, FixedDotWeight);

impl<'a> MakeShape for FixedDot<'a> {
    fn shape(&self) -> Shape {
        Shape::Dot(DotShape {
            c: self.weight().circle,
        })
    }
}

impl<'a> TraverseOutward for FixedDot<'a> {}
impl<'a> GetFirstRail for FixedDot<'a> {}

pub type LooseDot<'a> = GenericPrimitive<'a, LooseDotWeight>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a> LooseDot<'a> {
    pub fn seg(&self) -> Option<LooseSegIndex> {
        self.layout
            .graph
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .graph
                        .edge_weight(
                            self.layout
                                .graph
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    Label::Adjacent
                )
            })
            .filter(|ni| {
                matches!(
                    self.layout.graph.node_weight(*ni).unwrap(),
                    Weight::LooseSeg(..)
                )
            })
            .map(|ni| LooseSegIndex::new(ni))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.layout
            .graph
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .graph
                        .edge_weight(
                            self.layout
                                .graph
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    Label::Adjacent
                )
            })
            .filter(|ni| {
                matches!(
                    self.layout.graph.node_weight(*ni).unwrap(),
                    Weight::LooseBend(..)
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
            .unwrap()
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
impl_fixed_primitive!(FixedSeg, FixedSegWeight);

impl<'a> MakeShape for FixedSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.width(),
        })
    }
}

impl<'a> GetEnds<FixedDotIndex, FixedDotIndex> for FixedSeg<'a> {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
        let v = self.adjacents();
        (FixedDotIndex::new(v[0]), FixedDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<FixedDotIndex, FixedDotIndex> for FixedSeg<'a> {}

pub type LooseSeg<'a> = GenericPrimitive<'a, LooseSegWeight>;
impl_loose_primitive!(LooseSeg, LooseSegWeight);

impl<'a> MakeShape for LooseSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: match ends.0 {
                DotIndex::Fixed(dot) => self.primitive(dot).weight().circle.pos,
                DotIndex::Loose(dot) => self.primitive(dot).weight().circle.pos,
            },
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.width(),
        })
    }
}

impl<'a> GetWidth for LooseSeg<'a> {
    fn width(&self) -> f64 {
        self.primitive(self.ends().1).weight().width()
    }
}

impl<'a> GetEnds<DotIndex, LooseDotIndex> for LooseSeg<'a> {
    fn ends(&self) -> (DotIndex, LooseDotIndex) {
        let v = self.adjacents();
        if let Weight::FixedDot(..) = self.layout.graph.node_weight(v[0]).unwrap() {
            (FixedDotIndex::new(v[0]).into(), LooseDotIndex::new(v[1]))
        } else if let Weight::FixedDot(..) = self.layout.graph.node_weight(v[1]).unwrap() {
            (FixedDotIndex::new(v[1]).into(), LooseDotIndex::new(v[0]))
        } else {
            (LooseDotIndex::new(v[0]).into(), LooseDotIndex::new(v[1]))
        }
    }
}

impl<'a> GetOtherEnd<DotIndex, LooseDotIndex> for LooseSeg<'a> {}

pub type FixedBend<'a> = GenericPrimitive<'a, FixedBendWeight>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<'a> FixedBend<'a> {
    fn inner_radius(&self) -> f64 {
        todo!();
    }

    pub fn cross_product(&self) -> f64 {
        let center = self.primitive(self.core()).weight().circle.pos;
        let ends = self.ends();
        let end1 = self.primitive(ends.0).weight().circle.pos;
        let end2 = self.primitive(ends.1).weight().circle.pos;
        math::cross_product(end1 - center, end2 - center)
    }
}

impl<'a> MakeShape for FixedBend<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();

        let mut bend_shape = BendShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            c: Circle {
                pos: self.primitive(self.core()).weight().circle.pos,
                r: self.inner_radius(),
            },
            width: self.width(),
        };

        if self.weight().cw {
            swap(&mut bend_shape.from, &mut bend_shape.to);
        }
        Shape::Bend(bend_shape)
    }
}

impl<'a> GetEnds<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
        let v = self.adjacents();
        (FixedDotIndex::new(v[0]), FixedDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {}
impl<'a> TraverseOutward for FixedBend<'a> {}
impl<'a> GetFirstRail for FixedBend<'a> {}
impl<'a> GetCore for FixedBend<'a> {} // TODO: Fixed bends don't have cores actually.
                                      //impl<'a> GetInnerOuter for FixedBend<'a> {}

pub type LooseBend<'a> = GenericPrimitive<'a, LooseBendWeight>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<'a> LooseBend<'a> {
    fn inner_radius(&self) -> f64 {
        let mut r = 0.0;
        let mut rail = LooseBendIndex::new(self.index.node_index());

        while let Some(inner) = self.primitive(rail).inner() {
            r += self.primitive(inner).width();
            rail = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(LooseBendIndex::new(self.index.node_index()))
                    .core(),
            )
            .weight()
            .circle;

        core_circle.r + r + 3.0
    }
}

impl<'a> MakeShape for LooseBend<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();

        let mut bend_shape = BendShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            c: Circle {
                pos: self.primitive(self.core()).weight().circle.pos,
                r: self.inner_radius(),
            },
            width: self.width(),
        };

        if self.weight().cw {
            swap(&mut bend_shape.from, &mut bend_shape.to);
        }
        Shape::Bend(bend_shape)
    }
}

impl<'a> GetWidth for LooseBend<'a> {
    fn width(&self) -> f64 {
        self.primitive(self.ends().1).weight().width()
    }
}

impl<'a> GetEnds<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {
    fn ends(&self) -> (LooseDotIndex, LooseDotIndex) {
        let v = self.adjacents();
        (LooseDotIndex::new(v[0]), LooseDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {}
impl<'a> GetCore for LooseBend<'a> {}
impl<'a> GetInnerOuter for LooseBend<'a> {}
