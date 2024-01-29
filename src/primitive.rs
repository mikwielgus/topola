use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;
use petgraph::Direction::{Incoming, Outgoing};

use crate::connectivity::{BandIndex, ComponentIndex, GetNet};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::layout::dot::DotWeight;
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

use crate::shape::{Shape, ShapeTrait};

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
pub trait GetLimbs {
    fn limbs(&self) -> Vec<GeometryIndex> {
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

pub trait GetOtherJoint<F: GetNodeIndex, T: GetNodeIndex + Into<F>>: GetJoints<F, T> {
    fn other_joint(&self, end: F) -> F {
        let ends = self.joints();
        if ends.0.node_index() != end.node_index() {
            ends.0
        } else {
            ends.1.into()
        }
    }
}

pub trait GetJoints<F, T> {
    fn joints(&self) -> (F, T);
}

pub trait GetFirstRail: GetLayout + GetNodeIndex {
    fn first_rail(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .first_rail(self.node_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
    }
}

pub trait GetBendIndex {
    fn bend_index(&self) -> BendIndex;
}

pub trait GetCore: GetLayout + GetBendIndex {
    fn core(&self) -> FixedDotIndex {
        FixedDotIndex::new(
            self.layout()
                .geometry()
                .core(self.bend_index())
                .node_index(),
        )
    }
}

pub trait GetInnerOuter: GetLayout + GetBendIndex {
    fn inner(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.layout()
            .geometry()
            .outer(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
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

#[enum_dispatch(GetNet, GetWidth, GetLayout, GetConnectable, MakeShape, GetLimbs)]
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
        self.layout
            .geometry()
            .connecteds(self.index.into())
            .into_iter()
            .find_map(|ni| {
                let weight = self
                    .layout
                    .geometry()
                    .graph()
                    .node_weight(ni.node_index())
                    .unwrap();
                if matches!(weight, GeometryWeight::LoneLooseSeg(..)) {
                    Some(LoneLooseSegIndex::new(ni.node_index()).into())
                } else if matches!(weight, GeometryWeight::SeqLooseSeg(..)) {
                    Some(SeqLooseSegIndex::new(ni.node_index()).into())
                } else {
                    None
                }
            })
    }
}

impl<'a> MakeShape for FixedDot<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().dot_shape(self.index.into())
    }
}

impl<'a> GetLimbs for FixedDot<'a> {
    fn segs(&self) -> Vec<SegIndex> {
        self.layout
            .geometry()
            .connected_segs(self.index.into())
            .collect()
    }

    fn bends(&self) -> Vec<BendIndex> {
        self.layout
            .geometry()
            .connected_bends(self.index.into())
            .collect()
    }
}

impl<'a> GetFirstRail for FixedDot<'a> {}

pub type LooseDot<'a> = GenericPrimitive<'a, LooseDotWeight>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a> LooseDot<'a> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.layout
            .geometry()
            .connected_segs(self.index.into())
            .map(|ni| SeqLooseSegIndex::new(ni.node_index()))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.layout
            .geometry()
            .connected_bends(self.index.into())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
            .next()
            .unwrap()
    }
}

impl<'a> MakeShape for LooseDot<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().dot_shape(self.index.into())
    }
}

impl<'a> GetLimbs for LooseDot<'a> {
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
        self.layout.geometry().seg_shape(self.index.into())
    }
}

impl<'a> GetLimbs for FixedSeg<'a> {}

impl<'a> GetJoints<FixedDotIndex, FixedDotIndex> for FixedSeg<'a> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.layout.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a> GetOtherJoint<FixedDotIndex, FixedDotIndex> for FixedSeg<'a> {}

pub type LoneLooseSeg<'a> = GenericPrimitive<'a, LoneLooseSegWeight>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<'a> MakeShape for LoneLooseSeg<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().seg_shape(self.index.into())
    }
}

impl<'a> GetLimbs for LoneLooseSeg<'a> {}

impl<'a> GetJoints<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.layout.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a> GetOtherJoint<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a> {}

pub type SeqLooseSeg<'a> = GenericPrimitive<'a, SeqLooseSegWeight>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<'a> MakeShape for SeqLooseSeg<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().seg_shape(self.index.into())
    }
}

impl<'a> GetLimbs for SeqLooseSeg<'a> {}

impl<'a> GetJoints<DotIndex, LooseDotIndex> for SeqLooseSeg<'a> {
    fn joints(&self) -> (DotIndex, LooseDotIndex) {
        let joints = self.layout.geometry().seg_joints(self.index.into());
        if let DotWeight::Fixed(..) = self.layout.geometry().dot_weight(joints.0) {
            (
                FixedDotIndex::new(joints.0.node_index()).into(),
                LooseDotIndex::new(joints.1.node_index()).into(),
            )
        } else if let DotWeight::Fixed(..) = self.layout.geometry().dot_weight(joints.1) {
            (
                FixedDotIndex::new(joints.1.node_index()).into(),
                LooseDotIndex::new(joints.0.node_index()),
            )
        } else {
            (
                LooseDotIndex::new(joints.0.node_index()).into(),
                LooseDotIndex::new(joints.1.node_index()).into(),
            )
        }
    }
}

impl<'a> GetOtherJoint<DotIndex, LooseDotIndex> for SeqLooseSeg<'a> {}

pub type FixedBend<'a> = GenericPrimitive<'a, FixedBendWeight>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<'a> GetBendIndex for FixedBend<'a> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a> MakeShape for FixedBend<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().bend_shape(self.index.into())
    }
}

impl<'a> GetLimbs for FixedBend<'a> {}

impl<'a> GetJoints<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.layout.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a> GetOtherJoint<FixedDotIndex, FixedDotIndex> for FixedBend<'a> {}
impl<'a> GetFirstRail for FixedBend<'a> {}
impl<'a> GetCore for FixedBend<'a> {} // TODO: Fixed bends don't have cores actually.
                                      //impl<'a> GetInnerOuter for FixedBend<'a> {}

pub type LooseBend<'a> = GenericPrimitive<'a, LooseBendWeight>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<'a> GetBendIndex for LooseBend<'a> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a> From<LooseBend<'a>> for BendIndex {
    fn from(bend: LooseBend<'a>) -> BendIndex {
        bend.index.into()
    }
}

impl<'a> MakeShape for LooseBend<'a> {
    fn shape(&self) -> Shape {
        self.layout.geometry().bend_shape(self.index.into())
    }
}

impl<'a> GetLimbs for LooseBend<'a> {}

impl<'a> GetOffset for LooseBend<'a> {
    fn offset(&self) -> f64 {
        self.weight().offset
    }
}

impl<'a> GetJoints<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.layout.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.node_index()),
            LooseDotIndex::new(to.node_index()),
        )
    }
}

impl<'a> GetOtherJoint<LooseDotIndex, LooseDotIndex> for LooseBend<'a> {}
impl<'a> GetCore for LooseBend<'a> {}
impl<'a> GetInnerOuter for LooseBend<'a> {}
