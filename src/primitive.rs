use std::mem::swap;

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use petgraph::Direction::{Incoming, Outgoing};

use crate::connectivity::{BandIndex, ComponentIndex, GetNet};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::layout::seg::{
    FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
    SeqLooseSegIndex, SeqLooseSegWeight,
};
use crate::layout::Layout;
use crate::layout::{
    bend::{BendIndex, FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    geometry::{
        GeometryIndex, GeometryLabel, GeometryWeight, GetBandIndex, GetComponentIndex, GetOffset,
        GetWidth, MakePrimitive, Retag,
    },
};
use crate::loose::{Loose, LooseIndex};
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

#[enum_dispatch]
pub trait GetLegs {
    fn legs(&self) -> Vec<GeometryIndex> {
        let mut v = vec![];
        v.extend(self.segs().into_iter().map(Into::<GeometryIndex>::into));
        v.extend(self.bends().into_iter().map(Into::<GeometryIndex>::into));
        v
    }

    fn segs(&self) -> Vec<SegIndex> {
        vec![]
    }

    fn bends(&self) -> Vec<BendIndex> {
        vec![]
    }
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

pub trait GetFirstRail: GetLayout + GetNodeIndex {
    fn first_rail(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .graph()
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|node| {
                matches!(
                    self.layout()
                        .geometry()
                        .graph()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .graph()
                                .find_edge(*node, self.node_index())
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|node| LooseBendIndex::new(node))
            .next()
    }
}

pub trait GetCore: GetLayout + GetNodeIndex {
    fn core(&self) -> FixedDotIndex {
        self.layout()
            .geometry()
            .graph()
            .neighbors(self.node_index())
            .filter(|node| {
                matches!(
                    self.layout()
                        .geometry()
                        .graph()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .graph()
                                .find_edge(self.node_index(), *node)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|node| FixedDotIndex::new(node))
            .next()
            .unwrap()
    }
}

pub trait GetInnerOuter: GetLayout + GetNodeIndex {
    fn inner(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .graph()
            .neighbors_directed(self.node_index(), Incoming)
            .filter(|node| {
                matches!(
                    self.layout()
                        .geometry()
                        .graph()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .graph()
                                .find_edge(*node, self.node_index())
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|node| LooseBendIndex::new(node))
            .next()
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .graph()
            .neighbors_directed(self.node_index(), Outgoing)
            .filter(|node| {
                matches!(
                    self.layout()
                        .geometry()
                        .graph()
                        .edge_weight(
                            self.layout()
                                .geometry()
                                .graph()
                                .find_edge(self.node_index(), *node)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .map(|node| LooseBendIndex::new(node))
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

        impl<'a> GetComponentIndex for $primitive_struct<'a> {
            fn component(&self) -> ComponentIndex {
                self.weight().component()
            }
        }

        impl<'a> GetNet for $primitive_struct<'a> {
            fn net(&self) -> i64 {
                self.layout()
                    .connectivity()
                    .node_weight(self.component().node_index())
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

#[enum_dispatch(GetNet, GetWidth, GetLayout, GetConnectable, MakeShape, GetLegs)]
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
            .graph()
            .node_weight(self.index.node_index())
            .unwrap()
    }

    fn adjacents(&self) -> Vec<NodeIndex<usize>> {
        self.layout
            .geometry()
            .graph()
            .neighbors_undirected(self.index.node_index())
            .filter(|node| {
                matches!(
                    self.layout
                        .geometry()
                        .graph()
                        .edge_weight(
                            self.layout
                                .geometry()
                                .graph()
                                .find_edge_undirected(self.index.node_index(), *node)
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

impl<'a> FixedDot<'a> {
    pub fn first_loose(&self, _band: BandIndex) -> Option<LooseIndex> {
        self.adjacents().into_iter().find_map(|node| {
            let weight = self.layout.geometry().graph().node_weight(node).unwrap();
            if matches!(weight, GeometryWeight::LoneLooseSeg(..)) {
                Some(LoneLooseSegIndex::new(node).into())
            } else if matches!(weight, GeometryWeight::SeqLooseSeg(..)) {
                Some(SeqLooseSegIndex::new(node).into())
            } else {
                None
            }
        })
    }
}

impl<'a> MakeShape for FixedDot<'a> {
    fn shape(&self) -> Shape {
        Shape::Dot(DotShape {
            c: self.weight().circle,
        })
    }
}

impl<'a> GetLegs for FixedDot<'a> {
    fn segs(&self) -> Vec<SegIndex> {
        self.adjacents()
            .into_iter()
            .filter_map(
                |node| match self.layout.geometry().graph().node_weight(node).unwrap() {
                    GeometryWeight::FixedSeg(_seg) => {
                        Some(SegIndex::Fixed(FixedSegIndex::new(node)))
                    }
                    GeometryWeight::LoneLooseSeg(_seg) => {
                        Some(SegIndex::LoneLoose(LoneLooseSegIndex::new(node).into()))
                    }
                    GeometryWeight::SeqLooseSeg(_seg) => {
                        Some(SegIndex::SeqLoose(SeqLooseSegIndex::new(node).into()))
                    }
                    _ => None,
                },
            )
            .collect()
    }

    fn bends(&self) -> Vec<BendIndex> {
        self.adjacents()
            .into_iter()
            .filter(|node| {
                matches!(
                    self.layout.geometry().graph().node_weight(*node).unwrap(),
                    GeometryWeight::FixedBend(..)
                )
            })
            .map(|node| FixedBendIndex::new(node).into())
            .collect()
    }
}

impl<'a> GetFirstRail for FixedDot<'a> {}

pub type LooseDot<'a> = GenericPrimitive<'a, LooseDotWeight>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a> LooseDot<'a> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.adjacents()
            .into_iter()
            .filter(|node| {
                matches!(
                    self.layout.geometry().graph().node_weight(*node).unwrap(),
                    GeometryWeight::SeqLooseSeg(..)
                )
            })
            .map(|node| SeqLooseSegIndex::new(node))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.adjacents()
            .into_iter()
            .filter(|node| {
                matches!(
                    self.layout.geometry().graph().node_weight(*node).unwrap(),
                    GeometryWeight::LooseBend(..)
                )
            })
            .map(|node| LooseBendIndex::new(node))
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

impl<'a> GetLegs for LooseDot<'a> {
    fn segs(&self) -> Vec<SegIndex> {
        if let Some(seg) = self.seg() {
            vec![seg.into()]
        } else {
            vec![]
        }
    }

    fn bends(&self) -> Vec<BendIndex> {
        vec![self.bend().into()]
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

impl<'a> GetLegs for FixedSeg<'a> {}

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

impl<'a> GetLegs for LoneLooseSeg<'a> {}

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

impl<'a> GetLegs for SeqLooseSeg<'a> {}

impl<'a> GetWidth for SeqLooseSeg<'a> {
    fn width(&self) -> f64 {
        self.primitive(self.ends().1).weight().width()
    }
}

impl<'a> GetEnds<DotIndex, LooseDotIndex> for SeqLooseSeg<'a> {
    fn ends(&self) -> (DotIndex, LooseDotIndex) {
        let v = self.adjacents();
        if let GeometryWeight::FixedDot(..) =
            self.layout.geometry().graph().node_weight(v[0]).unwrap()
        {
            (FixedDotIndex::new(v[0]).into(), LooseDotIndex::new(v[1]))
        } else if let GeometryWeight::FixedDot(..) =
            self.layout.geometry().graph().node_weight(v[1]).unwrap()
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

impl<'a> GetLegs for FixedBend<'a> {}

impl<'a> GetEnds<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {
    fn ends(&self) -> (FixedDotIndex, FixedDotIndex) {
        let v = self.adjacents();
        (FixedDotIndex::new(v[0]), FixedDotIndex::new(v[1]))
    }
}

impl<'a> GetOtherEnd<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {}
impl<'a> GetFirstRail for FixedBend<'a> {}
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

impl<'a> GetLegs for LooseBend<'a> {}

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
impl<'a> GetCore for LooseBend<'a> {}
impl<'a> GetInnerOuter for LooseBend<'a> {}
