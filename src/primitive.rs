use std::mem::swap;

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use petgraph::Direction::{Incoming, Outgoing};

use crate::connectivity::GetNet;
use crate::geometry::{
    DotIndex, FixedBendWeight, FixedDotIndex, FixedDotWeight, FixedSegWeight, GeometryIndex,
    GeometryLabel, GeometryWeight, GetBandIndex, GetComponentIndex, GetOffset, GetWidth,
    LoneLooseSegWeight, LooseBendIndex, LooseBendWeight, LooseDotIndex, LooseDotWeight,
    MakePrimitive, Retag, SeqLooseSegIndex, SeqLooseSegWeight,
};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::layout::Layout;
use crate::math::{self, Circle};
use crate::shape::{BendShape, DotShape, SegShape, Shape, ShapeTrait};

#[enum_dispatch]
pub trait GetLayout {
    fn layout(&self) -> &Layout;
}

#[enum_dispatch]
pub trait GetConnectable: GetNet + GetLayout {
    fn connectable(&self, node: GeometryIndex) -> bool {
        let this = self.net();
        let other = node.primitive(self.layout()).net();

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

pub trait GetInterior<T> {
    fn interior(&self) -> Vec<T>;
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

pub trait GetEnds<F, T> {
    fn ends(&self) -> (F, T);
}

#[enum_dispatch]
pub trait GetWraparound: GetLayout + GetNodeIndex {
    fn wraparound(&self) -> Option<LooseBendIndex>;
}

pub trait GetFirstRail: GetLayout + GetNodeIndex {
    fn first_rail(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|ni| {
                matches!(
                    self.layout()
                        .geometry()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .find_edge(*ni, self.node_index())
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }
}

pub trait GetCore: GetLayout + GetNodeIndex {
    fn core(&self) -> FixedDotIndex {
        self.layout()
            .geometry()
            .neighbors(self.node_index())
            .filter(|ni| {
                matches!(
                    self.layout()
                        .geometry()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .find_edge(self.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
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
            .geometry()
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|ni| {
                matches!(
                    self.layout()
                        .geometry()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .find_edge(*ni, self.node_index())
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|ni| LooseBendIndex::new(ni))
            .next()
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .neighbors_directed(self.node_index(), Outgoing)
            .filter(|ni| {
                matches!(
                    self.layout()
                        .geometry()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .find_edge(self.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Outer
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
                if let GeometryWeight::$primitive_struct(weight) = self.tagged_weight() {
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
                self.layout()
                    .connectivity()
                    .node_weight(self.weight().component().node_index())
                    .unwrap()
                    .net()
            }
        }
    };
}

macro_rules! impl_loose_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_struct, $weight_struct);

        impl<'a> GetNet for $primitive_struct<'a> {
            fn net(&self) -> i64 {
                self.layout()
                    .connectivity()
                    .node_weight(self.weight().band().node_index())
                    .unwrap()
                    .net()
            }
        }
    };
}

#[enum_dispatch(GetNet, GetWidth, GetLayout, GetConnectable, MakeShape)]
pub enum Primitive<'a> {
    FixedDot(FixedDot<'a>),
    LooseDot(LooseDot<'a>),
    FixedSeg(FixedSeg<'a>),
    LoneLooseSeg(LoneLooseSeg<'a>),
    SeqLooseSeg(SeqLooseSeg<'a>),
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

    fn tagged_weight(&self) -> GeometryWeight {
        *self
            .layout
            .geometry()
            .node_weight(self.index.node_index())
            .unwrap()
    }

    fn adjacents(&self) -> Vec<NodeIndex<usize>> {
        self.layout
            .geometry()
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .geometry()
                        .edge_weight(
                            self.layout
                                .geometry()
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Adjacent
                )
            })
            .collect()
    }

    fn primitive<WW>(&self, index: GenericIndex<WW>) -> GenericPrimitive<WW> {
        GenericPrimitive::new(index, &self.layout)
    }
}

impl<'a, W> GetInterior<GeometryIndex> for GenericPrimitive<'a, W> {
    fn interior(&self) -> Vec<GeometryIndex> {
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
impl<'a> GetFirstRail for FixedDot<'a> {}
impl<'a> GetWraparound for FixedDot<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}

pub type LooseDot<'a> = GenericPrimitive<'a, LooseDotWeight>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a> LooseDot<'a> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.layout
            .geometry()
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .geometry()
                        .edge_weight(
                            self.layout
                                .geometry()
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Adjacent
                )
            })
            .filter(|ni| {
                matches!(
                    self.layout.geometry().node_weight(*ni).unwrap(),
                    GeometryWeight::SeqLooseSeg(..)
                )
            })
            .map(|ni| SeqLooseSegIndex::new(ni))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.layout
            .geometry()
            .neighbors_undirected(self.index.node_index())
            .filter(|ni| {
                matches!(
                    self.layout
                        .geometry()
                        .edge_weight(
                            self.layout
                                .geometry()
                                .find_edge_undirected(self.index.node_index(), *ni)
                                .unwrap()
                                .0,
                        )
                        .unwrap(),
                    GeometryLabel::Adjacent
                )
            })
            .filter(|ni| {
                matches!(
                    self.layout.geometry().node_weight(*ni).unwrap(),
                    GeometryWeight::LooseBend(..)
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

pub type LoneLooseSeg<'a> = GenericPrimitive<'a, LoneLooseSegWeight>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<'a> MakeShape for LoneLooseSeg<'a> {
    fn shape(&self) -> Shape {
        let ends = self.ends();
        Shape::Seg(SegShape {
            from: self.primitive(ends.0).weight().circle.pos,
            to: self.primitive(ends.1).weight().circle.pos,
            width: self.width(),
        })
    }
}

impl<'a> GetWidth for LoneLooseSeg<'a> {
    fn width(&self) -> f64 {
        self.primitive(self.ends().1).weight().width()
    }
}

impl<'a> GetEnds<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a> {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
        let v = self.adjacents();
        (FixedDotIndex::new(v[0]), FixedDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a> {}

pub type SeqLooseSeg<'a> = GenericPrimitive<'a, SeqLooseSegWeight>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<'a> MakeShape for SeqLooseSeg<'a> {
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

impl<'a> GetWidth for SeqLooseSeg<'a> {
    fn width(&self) -> f64 {
        self.primitive(self.ends().1).weight().width()
    }
}

impl<'a> GetEnds<DotIndex, LooseDotIndex> for SeqLooseSeg<'a> {
    fn ends(&self) -> (DotIndex, LooseDotIndex) {
        let v = self.adjacents();
        if let GeometryWeight::FixedDot(..) = self.layout.geometry().node_weight(v[0]).unwrap() {
            (FixedDotIndex::new(v[0]).into(), LooseDotIndex::new(v[1]))
        } else if let GeometryWeight::FixedDot(..) =
            self.layout.geometry().node_weight(v[1]).unwrap()
        {
            (FixedDotIndex::new(v[1]).into(), LooseDotIndex::new(v[0]))
        } else {
            (LooseDotIndex::new(v[0]).into(), LooseDotIndex::new(v[1]))
        }
    }
}

impl<'a> GetOtherEnd<DotIndex, LooseDotIndex> for SeqLooseSeg<'a> {}

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
impl<'a> GetFirstRail for FixedBend<'a> {}
impl<'a> GetWraparound for FixedBend<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.first_rail()
    }
}
impl<'a> GetCore for FixedBend<'a> {} // TODO: Fixed bends don't have cores actually.
                                      //impl<'a> GetInnerOuter for FixedBend<'a> {}

pub type LooseBend<'a> = GenericPrimitive<'a, LooseBendWeight>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<'a> LooseBend<'a> {
    fn inner_radius(&self) -> f64 {
        let mut r = self.offset();
        let mut rail = LooseBendIndex::new(self.index.node_index());

        while let Some(inner) = self.primitive(rail).inner() {
            let primitive = self.primitive(inner);
            r += primitive.width() + primitive.offset();
            rail = inner;
        }

        let core_circle = self
            .primitive(
                self.primitive(LooseBendIndex::new(self.index.node_index()))
                    .core(),
            )
            .weight()
            .circle;

        core_circle.r + r
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

impl<'a> GetOffset for LooseBend<'a> {
    fn offset(&self) -> f64 {
        self.weight().offset
    }
}

impl<'a> GetEnds<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {
    fn ends(&self) -> (LooseDotIndex, LooseDotIndex) {
        let v = self.adjacents();
        (LooseDotIndex::new(v[0]), LooseDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {}
impl<'a> GetWraparound for LooseBend<'a> {
    fn wraparound(&self) -> Option<LooseBendIndex> {
        self.outer()
    }
}

impl<'a> GetCore for LooseBend<'a> {}
impl<'a> GetInnerOuter for LooseBend<'a> {}
