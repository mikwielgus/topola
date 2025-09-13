// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        graph::{GetMaybeNet, PrimitiveIndex, PrimitiveWeight},
        rules::{AccessRules, Conditions, GetConditions},
        seg::{FixedSegWeight, LoneLooseSegWeight, SegIndex, SeqLooseSegIndex, SeqLooseSegWeight},
        Drawing,
    },
    geometry::{primitive::PrimitiveShape, GenericNode, GetLayer, GetOffset, GetWidth, Retag},
    graph::{GenericIndex, GetIndex},
};

use super::gear::{DrawingOutwardWalker, GetOuterGears, WalkOutwards};

pub trait GetDrawing {
    type CompoundWeight;
    type CompoundEntryLabel;
    type Rules;
    fn drawing(&self) -> &Drawing<Self::CompoundWeight, Self::CompoundEntryLabel, Self::Rules>;
}

#[enum_dispatch]
pub trait GetWeight {
    type Weight;
    fn weight(&self) -> Self::Weight;
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

pub trait GetOtherJoint: GetJoints {
    type J;
    fn other_joint(&self, end: Self::J) -> Self::J;
}

impl<F, S> GetOtherJoint for S
where
    F: GetIndex,
    S: GetJoints<F = F>,
    <S as GetJoints>::T: GetIndex + Into<F>,
{
    type J = F;
    fn other_joint(&self, end: F) -> F {
        let joints = self.joints();
        if joints.0.index() != end.index() {
            joints.0
        } else {
            joints.1.into()
        }
    }
}

pub trait GetJoints {
    type F;
    type T;
    fn joints(&self) -> (Self::F, Self::T);
}

pub trait GetLowestGears: GetDrawing + GetIndex {
    // TODO: Make it return an iterator instead of a vec.
    fn lowest_gears(&self) -> Vec<LooseBendIndex> {
        self.drawing()
            .geometry()
            .all_rails(self.index())
            .map(|ni| LooseBendIndex::new(ni.index()))
            .collect()
    }
}

pub trait GetBendIndex {
    fn bend_index(&self) -> BendIndex;
}

pub trait GetCore: GetBendIndex {
    fn core(&self) -> FixedDotIndex;
}

impl<S: GetDrawing + GetBendIndex> GetCore for S {
    fn core(&self) -> FixedDotIndex {
        FixedDotIndex::new(self.drawing().geometry().core(self.bend_index()).index())
    }
}

macro_rules! impl_primitive {
    ($primitive_variant:ident, $primitive_refstruct:ident, $weight_struct:ident) => {
        impl<CW, Cel, R> GetWeight for $primitive_refstruct<'_, CW, Cel, R> {
            type Weight = $weight_struct;
            fn weight(&self) -> $weight_struct {
                if let PrimitiveWeight::$primitive_variant(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }

        impl<CW, Cel, R> GetLayer for $primitive_refstruct<'_, CW, Cel, R> {
            fn layer(&self) -> usize {
                self.weight().layer()
            }
        }

        impl<CW, Cel, R> GetMaybeNet for $primitive_refstruct<'_, CW, Cel, R> {
            fn maybe_net(&self) -> Option<usize> {
                self.weight().maybe_net()
            }
        }
    };
}

macro_rules! impl_fixed_primitive {
    ($primitive_variant:ident, $primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_variant, $primitive_struct, $weight_struct);
    };
}

macro_rules! impl_loose_primitive {
    ($primitive_variant:ident, $primitive_struct:ident, $weight_struct:ident) => {
        impl_primitive!($primitive_variant, $primitive_struct, $weight_struct);
    };
}

#[enum_dispatch(
    GetLayer,
    GetMaybeNet,
    GetWidth,
    GetDrawing,
    MakePrimitiveShape,
    GetLimbs
)]
pub enum PrimitiveRef<'a, CW, Cel, R> {
    FixedDot(FixedDotRef<'a, CW, Cel, R>),
    LooseDot(LooseDotRef<'a, CW, Cel, R>),
    FixedSeg(FixedSegRef<'a, CW, Cel, R>),
    LoneLooseSeg(LoneLooseSegRef<'a, CW, Cel, R>),
    SeqLooseSeg(SeqLooseSegRef<'a, CW, Cel, R>),
    FixedBend(FixedBendRef<'a, CW, Cel, R>),
    LooseBend(LooseBendRef<'a, CW, Cel, R>),
}

impl<'a, CW, Cel, R: AccessRules> GetConditions<'a> for &PrimitiveRef<'a, CW, Cel, R> {
    fn conditions(self) -> Option<Conditions<'a>> {
        match self {
            PrimitiveRef::FixedDot(x) => x.conditions(),
            PrimitiveRef::LooseDot(x) => x.conditions(),
            PrimitiveRef::FixedSeg(x) => x.conditions(),
            PrimitiveRef::LoneLooseSeg(x) => x.conditions(),
            PrimitiveRef::SeqLooseSeg(x) => x.conditions(),
            PrimitiveRef::FixedBend(x) => x.conditions(),
            PrimitiveRef::LooseBend(x) => x.conditions(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenericPrimitive<'a, W, CW, Cel, R> {
    pub index: GenericIndex<W>,
    drawing: &'a Drawing<CW, Cel, R>,
}

impl<'a, W, CW, Cel, R> GenericPrimitive<'a, W, CW, Cel, R> {
    pub fn new(index: GenericIndex<W>, drawing: &'a Drawing<CW, Cel, R>) -> Self {
        Self { index, drawing }
    }

    fn tagged_weight(&self) -> PrimitiveWeight {
        if let GenericNode::Primitive(weight) = self
            .drawing
            .geometry()
            .graph()
            .node_weight(self.index.index().into())
            .unwrap()
        {
            *weight
        } else {
            unreachable!()
        }
    }
}

impl<W, CW, Cel, R> GetInterior<PrimitiveIndex> for GenericPrimitive<'_, W, CW, Cel, R> {
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.tagged_weight().retag(self.index.index())]
    }
}

impl<W, CW, Cel, R> GetDrawing for GenericPrimitive<'_, W, CW, Cel, R> {
    type CompoundWeight = CW;
    type CompoundEntryLabel = Cel;
    type Rules = R;
    fn drawing(&self) -> &Drawing<CW, Cel, R> {
        self.drawing
    }
}

impl<W, CW, Cel, R> GetIndex for GenericPrimitive<'_, W, CW, Cel, R> {
    fn index(&self) -> usize {
        self.index.index()
    }
}

impl<'a, W: GetWidth, CW, Cel, R> GetWidth for GenericPrimitive<'a, W, CW, Cel, R>
where
    GenericPrimitive<'a, W, CW, Cel, R>: GetWeight<Weight = W>,
{
    fn width(&self) -> f64 {
        self.weight().width()
    }
}

impl<'a, W, CW, Cel, R> GetConditions<'a> for &GenericPrimitive<'a, W, CW, Cel, R>
where
    GenericPrimitive<'a, W, CW, Cel, R>: GetMaybeNet,
{
    fn conditions(self) -> Option<Conditions<'a>> {
        self.maybe_net().map(|net| Conditions {
            net,
            maybe_region: Some("A".into()),
            maybe_layer: Some("F.Cu".into()),
        })
    }
}

pub type FixedDotRef<'a, CW, Cel, R> = GenericPrimitive<'a, FixedDotWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedDot, FixedDotRef, FixedDotWeight);

impl<CW, Cel, R> MakePrimitiveShape for FixedDotRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedDotRef<'_, CW, Cel, R> {
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

impl<CW, Cel, R> GetLowestGears for FixedDotRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOuterGears for FixedDotRef<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.lowest_gears()
    }
}

impl<CW, Cel, R> WalkOutwards for FixedDotRef<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.lowest_gears().into_iter())
    }
}

pub type LooseDotRef<'a, CW, Cel, R> = GenericPrimitive<'a, LooseDotWeight, CW, Cel, R>;
impl_loose_primitive!(LooseDot, LooseDotRef, LooseDotWeight);

impl<CW, Cel, R> LooseDotRef<'_, CW, Cel, R> {
    pub fn seg(&self) -> Option<SeqLooseSegIndex> {
        self.drawing
            .geometry()
            .joined_segs(self.index.into())
            .map(|ni| SeqLooseSegIndex::new(ni.index()))
            .next()
    }

    pub fn bend(&self) -> LooseBendIndex {
        self.drawing
            .geometry()
            .joined_bends(self.index.into())
            .map(|ni| LooseBendIndex::new(ni.index()))
            .next()
            .unwrap()
    }
}

impl<CW, Cel, R> MakePrimitiveShape for LooseDotRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LooseDotRef<'_, CW, Cel, R> {
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

pub type FixedSegRef<'a, CW, Cel, R> = GenericPrimitive<'a, FixedSegWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedSeg, FixedSegRef, FixedSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for FixedSegRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedSegRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for FixedSegRef<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.index()),
            FixedDotIndex::new(to.index()),
        )
    }
}

pub type LoneLooseSegRef<'a, CW, Cel, R> = GenericPrimitive<'a, LoneLooseSegWeight, CW, Cel, R>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegRef, LoneLooseSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for LoneLooseSegRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LoneLooseSegRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for LoneLooseSegRef<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.index()),
            FixedDotIndex::new(to.index()),
        )
    }
}

pub type SeqLooseSegRef<'a, CW, Cel, R> = GenericPrimitive<'a, SeqLooseSegWeight, CW, Cel, R>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegRef, SeqLooseSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for SeqLooseSegRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for SeqLooseSegRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for SeqLooseSegRef<'_, CW, Cel, R> {
    type F = DotIndex;
    type T = LooseDotIndex;
    fn joints(&self) -> (DotIndex, LooseDotIndex) {
        let joints = self.drawing.geometry().seg_joints(self.index.into());
        if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.0) {
            (
                FixedDotIndex::new(joints.0.index()).into(),
                LooseDotIndex::new(joints.1.index()),
            )
        } else if let DotWeight::Fixed(..) = self.drawing.geometry().dot_weight(joints.1) {
            (
                FixedDotIndex::new(joints.1.index()).into(),
                LooseDotIndex::new(joints.0.index()),
            )
        } else {
            (
                LooseDotIndex::new(joints.0.index()).into(),
                LooseDotIndex::new(joints.1.index()),
            )
        }
    }
}

pub type FixedBendRef<'a, CW, Cel, R> = GenericPrimitive<'a, FixedBendWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedBend, FixedBendRef, FixedBendWeight);

impl<CW, Cel, R> GetBendIndex for FixedBendRef<'_, CW, Cel, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<CW, Cel, R> MakePrimitiveShape for FixedBendRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedBendRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for FixedBendRef<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.index()),
            FixedDotIndex::new(to.index()),
        )
    }
}

impl<CW, Cel, R> GetLowestGears for FixedBendRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOuterGears for FixedBendRef<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.lowest_gears()
    }
}

impl<CW, Cel, R> WalkOutwards for FixedBendRef<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.lowest_gears().into_iter())
    }
}

pub type LooseBendRef<'a, CW, Cel, R> = GenericPrimitive<'a, LooseBendWeight, CW, Cel, R>;
impl_loose_primitive!(LooseBend, LooseBendRef, LooseBendWeight);

impl<CW, Cel, R> GetBendIndex for LooseBendRef<'_, CW, Cel, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, CW: Clone, Cel: Copy, R: AccessRules> From<LooseBendRef<'a, CW, Cel, R>> for BendIndex {
    fn from(bend: LooseBendRef<'a, CW, Cel, R>) -> BendIndex {
        bend.index.into()
    }
}

impl<CW, Cel, R> MakePrimitiveShape for LooseBendRef<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LooseBendRef<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOffset for LooseBendRef<'_, CW, Cel, R> {
    fn offset(&self) -> f64 {
        self.weight().offset()
    }
}

impl<CW, Cel, R> GetJoints for LooseBendRef<'_, CW, Cel, R> {
    type F = LooseDotIndex;
    type T = LooseDotIndex;
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.index()),
            LooseDotIndex::new(to.index()),
        )
    }
}

impl<CW, Cel, R> GetOuterGears for LooseBendRef<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.outers().collect()
    }
}

impl<CW, Cel, R> WalkOutwards for LooseBendRef<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.outers())
    }
}

impl<CW, Cel, R> LooseBendRef<'_, CW, Cel, R> {
    pub fn inner(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.index()))
    }

    pub fn outers(&self) -> impl Iterator<Item = LooseBendIndex> + '_ {
        self.drawing()
            .geometry()
            .outers(self.bend_index())
            .map(|node| LooseBendIndex::new(node.index()))
    }
}
