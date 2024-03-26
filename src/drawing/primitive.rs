use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::geometry::{
    shape::{Shape, ShapeTrait},
    GetOffset, GetWidth,
};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::layout::connectivity::{BandIndex, ContinentIndex};
use crate::{
    drawing::{
        bend::{BendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetLayer, GetMaybeNet, PrimitiveIndex, PrimitiveWeight, Retag},
        loose::LooseIndex,
        rules::{Conditions, GetConditions, RulesTrait},
        seg::{
            FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex, SeqLooseSegIndex,
            SeqLooseSegWeight,
        },
        Drawing,
    },
    geometry::Compound,
};

#[enum_dispatch]
pub trait GetDrawing<'a, R: RulesTrait> {
    fn drawing(&self) -> &Drawing<R>;
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
    fn limbs(&self) -> Vec<PrimitiveIndex> {
        let mut v = vec![];
        v.extend(self.segs().into_iter().map(Into::<PrimitiveIndex>::into));
        v.extend(self.bends().into_iter().map(Into::<PrimitiveIndex>::into));
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

pub trait GetFirstRail<'a, R: RulesTrait>: GetDrawing<'a, R> + GetNodeIndex {
    fn first_rail(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .first_rail(self.node_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
    }
}

pub trait GetBendIndex {
    fn bend_index(&self) -> BendIndex;
}

pub trait GetCore<'a, R: RulesTrait>: GetDrawing<'a, R> + GetBendIndex {
    fn core(&self) -> FixedDotIndex {
        FixedDotIndex::new(
            self.drawing()
                .geometry()
                .core(self.bend_index())
                .node_index(),
        )
    }
}

pub trait GetInnerOuter<'a, R: RulesTrait>: GetDrawing<'a, R> + GetBendIndex {
    fn inner(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .outer(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
    }
}

macro_rules! impl_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl<'a, R: RulesTrait> GetWeight<$weight_struct> for $primitive_struct<'a, R> {
            fn weight(&self) -> $weight_struct {
                if let PrimitiveWeight::$primitive_struct(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }

        impl<'a, R: RulesTrait> GetLayer for $primitive_struct<'a, R> {
            fn layer(&self) -> u64 {
                self.weight().layer()
            }
        }

        impl<'a, R: RulesTrait> GetMaybeNet for $primitive_struct<'a, R> {
            fn maybe_net(&self) -> Option<usize> {
                self.weight().maybe_net()
            }
        }
    };
}

macro_rules! impl_fixed_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_struct, $weight_struct);
    };
}

macro_rules! impl_loose_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_struct, $weight_struct);
    };
}

#[enum_dispatch(
    GetLayer,
    GetMaybeNet,
    GetWidth,
    GetLayout,
    MakeShape,
    GetLimbs,
    GetConditions
)]
pub enum Primitive<'a, R: RulesTrait> {
    FixedDot(FixedDot<'a, R>),
    LooseDot(LooseDot<'a, R>),
    FixedSeg(FixedSeg<'a, R>),
    LoneLooseSeg(LoneLooseSeg<'a, R>),
    SeqLooseSeg(SeqLooseSeg<'a, R>),
    FixedBend(FixedBend<'a, R>),
    LooseBend(LooseBend<'a, R>),
}

#[derive(Debug)]
pub struct GenericPrimitive<'a, W, R: RulesTrait> {
    pub index: GenericIndex<W>,
    drawing: &'a Drawing<R>,
}

impl<'a, W, R: RulesTrait> GenericPrimitive<'a, W, R> {
    pub fn new(index: GenericIndex<W>, drawing: &'a Drawing<R>) -> Self {
        Self { index, drawing }
    }

    fn tagged_weight(&self) -> PrimitiveWeight {
        if let Compound::Primitive(weight) = *self
            .drawing
            .geometry()
            .graph()
            .node_weight(self.index.node_index())
            .unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn primitive<WW>(&self, index: GenericIndex<WW>) -> GenericPrimitive<WW, R> {
        GenericPrimitive::new(index, &self.drawing)
    }
}

impl<'a, W, R: RulesTrait> GetInterior<PrimitiveIndex> for GenericPrimitive<'a, W, R> {
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.tagged_weight().retag(self.index.node_index())]
    }
}

impl<'a, W, R: RulesTrait> GetDrawing<'a, R> for GenericPrimitive<'a, W, R> {
    fn drawing(&self) -> &Drawing<R> {
        self.drawing
    }
}

impl<'a, W, R: RulesTrait> GetNodeIndex for GenericPrimitive<'a, W, R> {
    fn node_index(&self) -> NodeIndex<usize> {
        self.index.node_index()
    }
}

impl<'a, W: GetWidth, R: RulesTrait> GetWidth for GenericPrimitive<'a, W, R>
where
    GenericPrimitive<'a, W, R>: GetWeight<W>,
{
    fn width(&self) -> f64 {
        self.weight().width()
    }
}

impl<'a, W, R: RulesTrait> GetConditions for GenericPrimitive<'a, W, R>
where
    GenericPrimitive<'a, W, R>: GetMaybeNet,
{
    fn conditions(&self) -> Conditions {
        Conditions {
            maybe_net: self.maybe_net(),
            maybe_region: Some("A".to_string()),
            maybe_layer: Some("F.Cu".to_string()),
        }
    }
}

pub type FixedDot<'a, R> = GenericPrimitive<'a, FixedDotWeight, R>;
impl_fixed_primitive!(FixedDot, FixedDotWeight);

impl<'a, R: RulesTrait> FixedDot<'a, R> {
    pub fn first_loose(&self, _band: BandIndex) -> Option<LooseIndex> {
        self.drawing
            .geometry()
            .joineds(self.index.into())
            .into_iter()
            .find_map(|ni| {
                let weight = self
                    .drawing
                    .geometry()
                    .graph()
                    .node_weight(ni.node_index())
                    .unwrap();
                if matches!(
                    weight,
                    Compound::Primitive(PrimitiveWeight::LoneLooseSeg(..))
                ) {
                    Some(LoneLooseSegIndex::new(ni.node_index()).into())
                } else if matches!(
                    weight,
                    Compound::Primitive(PrimitiveWeight::SeqLooseSeg(..))
                ) {
                    Some(SeqLooseSegIndex::new(ni.node_index()).into())
                } else {
                    None
                }
            })
    }
}

impl<'a, R: RulesTrait> MakeShape for FixedDot<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for FixedDot<'a, R> {
    fn segs(&self) -> Vec<SegIndex> {
        self.drawing
            .geometry()
            .joined_segs(self.index.into())
            .collect()
    }

    fn bends(&self) -> Vec<BendIndex> {
        self.drawing
            .geometry()
            .joined_bends(self.index.into())
            .collect()
    }
}

impl<'a, R: RulesTrait> GetFirstRail<'a, R> for FixedDot<'a, R> {}

pub type LooseDot<'a, R> = GenericPrimitive<'a, LooseDotWeight, R>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a, R: RulesTrait> LooseDot<'a, R> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.drawing
            .geometry()
            .joined_segs(self.index.into())
            .map(|ni| SeqLooseSegIndex::new(ni.node_index()))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.drawing
            .geometry()
            .joined_bends(self.index.into())
            .map(|ni| LooseBendIndex::new(ni.node_index()))
            .next()
            .unwrap()
    }
}

impl<'a, R: RulesTrait> MakeShape for LooseDot<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for LooseDot<'a, R> {
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

pub type FixedSeg<'a, R> = GenericPrimitive<'a, FixedSegWeight, R>;
impl_fixed_primitive!(FixedSeg, FixedSegWeight);

impl<'a, R: RulesTrait> MakeShape for FixedSeg<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for FixedSeg<'a, R> {}

impl<'a, R: RulesTrait> GetJoints<FixedDotIndex, FixedDotIndex> for FixedSeg<'a, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a, R: RulesTrait> GetOtherJoint<FixedDotIndex, FixedDotIndex> for FixedSeg<'a, R> {}

pub type LoneLooseSeg<'a, R> = GenericPrimitive<'a, LoneLooseSegWeight, R>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<'a, R: RulesTrait> MakeShape for LoneLooseSeg<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for LoneLooseSeg<'a, R> {}

impl<'a, R: RulesTrait> GetJoints<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a, R: RulesTrait> GetOtherJoint<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'a, R> {}

pub type SeqLooseSeg<'a, R> = GenericPrimitive<'a, SeqLooseSegWeight, R>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<'a, R: RulesTrait> MakeShape for SeqLooseSeg<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for SeqLooseSeg<'a, R> {}

impl<'a, R: RulesTrait> GetJoints<DotIndex, LooseDotIndex> for SeqLooseSeg<'a, R> {
    fn joints(&self) -> (DotIndex, LooseDotIndex) {
        let joints = self.drawing.geometry().seg_joints(self.index.into());
        if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.0) {
            (
                FixedDotIndex::new(joints.0.node_index()).into(),
                LooseDotIndex::new(joints.1.node_index()).into(),
            )
        } else if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.1) {
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

impl<'a, R: RulesTrait> GetOtherJoint<DotIndex, LooseDotIndex> for SeqLooseSeg<'a, R> {}

pub type FixedBend<'a, R> = GenericPrimitive<'a, FixedBendWeight, R>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<'a, R: RulesTrait> GetBendIndex for FixedBend<'a, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, R: RulesTrait> MakeShape for FixedBend<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for FixedBend<'a, R> {}

impl<'a, R: RulesTrait> GetJoints<FixedDotIndex, FixedDotIndex> for FixedBend<'a, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.node_index()),
            FixedDotIndex::new(to.node_index()),
        )
    }
}

impl<'a, R: RulesTrait> GetOtherJoint<FixedDotIndex, FixedDotIndex> for FixedBend<'a, R> {}
impl<'a, R: RulesTrait> GetFirstRail<'a, R> for FixedBend<'a, R> {}
impl<'a, R: RulesTrait> GetCore<'a, R> for FixedBend<'a, R> {} // TODO: Fixed bends don't have cores actually.
                                                               //impl<'a, R: QueryRules> GetInnerOuter for FixedBend<'a, R> {}

pub type LooseBend<'a, R> = GenericPrimitive<'a, LooseBendWeight, R>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<'a, R: RulesTrait> GetBendIndex for LooseBend<'a, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, R: RulesTrait> From<LooseBend<'a, R>> for BendIndex {
    fn from(bend: LooseBend<'a, R>) -> BendIndex {
        bend.index.into()
    }
}

impl<'a, R: RulesTrait> MakeShape for LooseBend<'a, R> {
    fn shape(&self) -> Shape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<'a, R: RulesTrait> GetLimbs for LooseBend<'a, R> {}

impl<'a, R: RulesTrait> GetOffset for LooseBend<'a, R> {
    fn offset(&self) -> f64 {
        self.weight().offset
    }
}

impl<'a, R: RulesTrait> GetJoints<LooseDotIndex, LooseDotIndex> for LooseBend<'a, R> {
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.node_index()),
            LooseDotIndex::new(to.node_index()),
        )
    }
}

impl<'a, R: RulesTrait> GetOtherJoint<LooseDotIndex, LooseDotIndex> for LooseBend<'a, R> {}
impl<'a, R: RulesTrait> GetCore<'a, R> for LooseBend<'a, R> {}
impl<'a, R: RulesTrait> GetInnerOuter<'a, R> for LooseBend<'a, R> {}
