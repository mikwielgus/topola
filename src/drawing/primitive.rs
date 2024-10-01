use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetLayer, GetMaybeNet, PrimitiveIndex, PrimitiveWeight, Retag},
        rules::{AccessRules, Conditions, GetConditions},
        seg::{FixedSegWeight, LoneLooseSegWeight, SegIndex, SeqLooseSegIndex, SeqLooseSegWeight},
        Drawing,
    },
    geometry::{primitive::PrimitiveShape, GenericNode, GetOffset, GetWidth},
    graph::{GenericIndex, GetPetgraphIndex},
};

#[enum_dispatch]
pub trait GetDrawing<'a, R: AccessRules> {
    fn drawing(&self) -> &Drawing<impl Copy, R>;
}

#[enum_dispatch]
pub trait GetWeight<W> {
    fn weight(&self) -> W;
}

#[enum_dispatch]
pub trait MakePrimitiveShape {
    fn shape(&self) -> PrimitiveShape;
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

pub trait GetOtherJoint<F: GetPetgraphIndex, T: GetPetgraphIndex + Into<F>>:
    GetJoints<F, T>
{
    fn other_joint(&self, end: F) -> F {
        let joints = self.joints();
        if joints.0.petgraph_index() != end.petgraph_index() {
            joints.0
        } else {
            joints.1.into()
        }
    }
}

pub trait GetJoints<F, T> {
    fn joints(&self) -> (F, T);
}

pub trait GetFirstGear<'a, R: AccessRules>: GetDrawing<'a, R> + GetPetgraphIndex {
    fn first_gear(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .first_rail(self.petgraph_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }
}

pub trait GetBendIndex {
    fn bend_index(&self) -> BendIndex;
}

pub trait GetCore<'a, R: AccessRules>: GetDrawing<'a, R> + GetBendIndex {
    fn core(&self) -> FixedDotIndex {
        FixedDotIndex::new(
            self.drawing()
                .geometry()
                .core(self.bend_index())
                .petgraph_index(),
        )
    }
}

pub trait GetInnerOuter<'a, R: AccessRules>: GetDrawing<'a, R> + GetBendIndex {
    fn inner(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }

    fn outer(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .outer(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }
}

macro_rules! impl_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl<'a, CW: Copy, R: AccessRules> GetWeight<$weight_struct>
            for $primitive_struct<'a, CW, R>
        {
            fn weight(&self) -> $weight_struct {
                if let PrimitiveWeight::$primitive_struct(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }

        impl<'a, CW: Copy, R: AccessRules> GetLayer for $primitive_struct<'a, CW, R> {
            fn layer(&self) -> usize {
                self.weight().layer()
            }
        }

        impl<'a, CW: Copy, R: AccessRules> GetMaybeNet for $primitive_struct<'a, CW, R> {
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
    GetDrawing,
    MakePrimitiveShape,
    GetLimbs,
    GetConditions
)]
pub enum Primitive<'a, CW: Copy, R: AccessRules> {
    FixedDot(FixedDot<'a, CW, R>),
    LooseDot(LooseDot<'a, CW, R>),
    FixedSeg(FixedSeg<'a, CW, R>),
    LoneLooseSeg(LoneLooseSeg<'a, CW, R>),
    SeqLooseSeg(SeqLooseSeg<'a, CW, R>),
    FixedBend(FixedBend<'a, CW, R>),
    LooseBend(LooseBend<'a, CW, R>),
}

#[derive(Debug)]
pub struct GenericPrimitive<'a, W, CW: Copy, R: AccessRules> {
    pub index: GenericIndex<W>,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, W, CW: Copy, R: AccessRules> GenericPrimitive<'a, W, CW, R> {
    pub fn new(index: GenericIndex<W>, drawing: &'a Drawing<CW, R>) -> Self {
        Self { index, drawing }
    }

    fn tagged_weight(&self) -> PrimitiveWeight {
        if let GenericNode::Primitive(weight) = *self
            .drawing
            .geometry()
            .graph()
            .node_weight(self.index.petgraph_index())
            .unwrap()
        {
            weight
        } else {
            unreachable!()
        }
    }

    fn primitive<WW>(&self, index: GenericIndex<WW>) -> GenericPrimitive<WW, CW, R> {
        GenericPrimitive::new(index, self.drawing)
    }
}

impl<'a, W, CW: Copy, R: AccessRules> GetInterior<PrimitiveIndex>
    for GenericPrimitive<'a, W, CW, R>
{
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.tagged_weight().retag(self.index.petgraph_index())]
    }
}

impl<'a, W, CW: Copy, R: AccessRules> GetDrawing<'a, R> for GenericPrimitive<'a, W, CW, R> {
    fn drawing(&self) -> &Drawing<impl Copy, R> {
        self.drawing
    }
}

impl<'a, W, CW: Copy, R: AccessRules> GetPetgraphIndex for GenericPrimitive<'a, W, CW, R> {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.index.petgraph_index()
    }
}

impl<'a, W: GetWidth, CW: Copy, R: AccessRules> GetWidth for GenericPrimitive<'a, W, CW, R>
where
    GenericPrimitive<'a, W, CW, R>: GetWeight<W>,
{
    fn width(&self) -> f64 {
        self.weight().width()
    }
}

impl<'a, W, CW: Copy, R: AccessRules> GetConditions for GenericPrimitive<'a, W, CW, R>
where
    GenericPrimitive<'a, W, CW, R>: GetMaybeNet,
{
    fn conditions(&self) -> Conditions {
        Conditions {
            maybe_net: self.maybe_net(),
            maybe_region: Some("A".to_string()),
            maybe_layer: Some("F.Cu".to_string()),
        }
    }
}

pub type FixedDot<'a, CW, R> = GenericPrimitive<'a, FixedDotWeight, CW, R>;
impl_fixed_primitive!(FixedDot, FixedDotWeight);

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for FixedDot<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for FixedDot<'a, CW, R> {
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

impl<'a, CW: Copy, R: AccessRules> GetFirstGear<'a, R> for FixedDot<'a, CW, R> {}

pub type LooseDot<'a, CW, R> = GenericPrimitive<'a, LooseDotWeight, CW, R>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<'a, CW: Copy, R: AccessRules> LooseDot<'a, CW, R> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.drawing
            .geometry()
            .joined_segs(self.index.into())
            .map(|ni| SeqLooseSegIndex::new(ni.petgraph_index()))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.drawing
            .geometry()
            .joined_bends(self.index.into())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
            .next()
            .unwrap()
    }
}

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for LooseDot<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for LooseDot<'a, CW, R> {
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

pub type FixedSeg<'a, CW, R> = GenericPrimitive<'a, FixedSegWeight, CW, R>;
impl_fixed_primitive!(FixedSeg, FixedSegWeight);

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for FixedSeg<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for FixedSeg<'a, CW, R> {}

impl<'a, CW: Copy, R: AccessRules> GetJoints<FixedDotIndex, FixedDotIndex> for FixedSeg<'a, CW, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<'a, CW: Copy, R: AccessRules> GetOtherJoint<FixedDotIndex, FixedDotIndex>
    for FixedSeg<'a, CW, R>
{
}

pub type LoneLooseSeg<'a, CW, R> = GenericPrimitive<'a, LoneLooseSegWeight, CW, R>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for LoneLooseSeg<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for LoneLooseSeg<'a, CW, R> {}

impl<'a, CW: Copy, R: AccessRules> GetJoints<FixedDotIndex, FixedDotIndex>
    for LoneLooseSeg<'a, CW, R>
{
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<'a, CW: Copy, R: AccessRules> GetOtherJoint<FixedDotIndex, FixedDotIndex>
    for LoneLooseSeg<'a, CW, R>
{
}

pub type SeqLooseSeg<'a, CW, R> = GenericPrimitive<'a, SeqLooseSegWeight, CW, R>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for SeqLooseSeg<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for SeqLooseSeg<'a, CW, R> {}

impl<'a, CW: Copy, R: AccessRules> GetJoints<DotIndex, LooseDotIndex> for SeqLooseSeg<'a, CW, R> {
    fn joints(&self) -> (DotIndex, LooseDotIndex) {
        let joints = self.drawing.geometry().seg_joints(self.index.into());
        if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.0) {
            (
                FixedDotIndex::new(joints.0.petgraph_index()).into(),
                LooseDotIndex::new(joints.1.petgraph_index()),
            )
        } else if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.1) {
            (
                FixedDotIndex::new(joints.1.petgraph_index()).into(),
                LooseDotIndex::new(joints.0.petgraph_index()),
            )
        } else {
            (
                LooseDotIndex::new(joints.0.petgraph_index()).into(),
                LooseDotIndex::new(joints.1.petgraph_index()),
            )
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetOtherJoint<DotIndex, LooseDotIndex>
    for SeqLooseSeg<'a, CW, R>
{
}

pub type FixedBend<'a, CW, R> = GenericPrimitive<'a, FixedBendWeight, CW, R>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<'a, CW: Copy, R: AccessRules> GetBendIndex for FixedBend<'a, CW, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for FixedBend<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for FixedBend<'a, CW, R> {}

impl<'a, CW: Copy, R: AccessRules> GetJoints<FixedDotIndex, FixedDotIndex>
    for FixedBend<'a, CW, R>
{
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<'a, CW: Copy, R: AccessRules> GetOtherJoint<FixedDotIndex, FixedDotIndex>
    for FixedBend<'a, CW, R>
{
}
impl<'a, CW: Copy, R: AccessRules> GetFirstGear<'a, R> for FixedBend<'a, CW, R> {}
impl<'a, CW: Copy, R: AccessRules> GetCore<'a, R> for FixedBend<'a, CW, R> {} // TODO: Fixed bends don't have cores actually.
                                                                              //impl<'a, R: QueryRules> GetInnerOuter for FixedBend<'a, CW, R> {}

pub type LooseBend<'a, CW, R> = GenericPrimitive<'a, LooseBendWeight, CW, R>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<'a, CW: Copy, R: AccessRules> GetBendIndex for LooseBend<'a, CW, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, CW: Copy, R: AccessRules> From<LooseBend<'a, CW, R>> for BendIndex {
    fn from(bend: LooseBend<'a, CW, R>) -> BendIndex {
        bend.index.into()
    }
}

impl<'a, CW: Copy, R: AccessRules> MakePrimitiveShape for LooseBend<'a, CW, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<'a, CW: Copy, R: AccessRules> GetLimbs for LooseBend<'a, CW, R> {}

impl<'a, CW: Copy, R: AccessRules> GetOffset for LooseBend<'a, CW, R> {
    fn offset(&self) -> f64 {
        self.weight().offset
    }
}

impl<'a, CW: Copy, R: AccessRules> GetJoints<LooseDotIndex, LooseDotIndex>
    for LooseBend<'a, CW, R>
{
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.petgraph_index()),
            LooseDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<'a, CW: Copy, R: AccessRules> GetOtherJoint<LooseDotIndex, LooseDotIndex>
    for LooseBend<'a, CW, R>
{
}
impl<'a, CW: Copy, R: AccessRules> GetCore<'a, R> for LooseBend<'a, CW, R> {}
impl<'a, CW: Copy, R: AccessRules> GetInnerOuter<'a, R> for LooseBend<'a, CW, R> {}
