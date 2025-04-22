// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

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
    graph::{GenericIndex, GetPetgraphIndex},
};

pub trait GetDrawing {
    type CompoundWeight;
    type CompoundEntryKind;
    type Rules;
    fn drawing(&self) -> &Drawing<Self::CompoundWeight, Self::CompoundEntryKind, Self::Rules>;
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

pub trait GetOtherJoint<F, T>: GetJoints<F, T> {
    fn other_joint(&self, end: F) -> F;
}

impl<F: GetPetgraphIndex, T: GetPetgraphIndex + Into<F>, S: GetJoints<F, T>> GetOtherJoint<F, T>
    for S
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

pub trait GetFirstGear: GetDrawing + GetPetgraphIndex {
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

pub trait GetCore: GetBendIndex {
    fn core(&self) -> FixedDotIndex;
}

impl<S: GetDrawing + GetBendIndex> GetCore for S {
    fn core(&self) -> FixedDotIndex {
        FixedDotIndex::new(
            self.drawing()
                .geometry()
                .core(self.bend_index())
                .petgraph_index(),
        )
    }
}

macro_rules! impl_primitive {
    ($primitive_struct:ident, $weight_struct:ident) => {
        impl<CW, Cek, R> GetWeight<$weight_struct> for $primitive_struct<'_, CW, Cek, R> {
            fn weight(&self) -> $weight_struct {
                if let PrimitiveWeight::$primitive_struct(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }

        impl<CW, Cek, R> GetLayer for $primitive_struct<'_, CW, Cek, R> {
            fn layer(&self) -> usize {
                self.weight().layer()
            }
        }

        impl<CW, Cek, R> GetMaybeNet for $primitive_struct<'_, CW, Cek, R> {
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
    GetLimbs
)]
pub enum Primitive<'a, CW, Cek, R> {
    FixedDot(FixedDot<'a, CW, Cek, R>),
    LooseDot(LooseDot<'a, CW, Cek, R>),
    FixedSeg(FixedSeg<'a, CW, Cek, R>),
    LoneLooseSeg(LoneLooseSeg<'a, CW, Cek, R>),
    SeqLooseSeg(SeqLooseSeg<'a, CW, Cek, R>),
    FixedBend(FixedBend<'a, CW, Cek, R>),
    LooseBend(LooseBend<'a, CW, Cek, R>),
}

impl<'a, CW, Cek, R: AccessRules> GetConditions<'a> for &Primitive<'a, CW, Cek, R> {
    fn conditions(self) -> Option<Conditions<'a>> {
        match self {
            Primitive::FixedDot(x) => x.conditions(),
            Primitive::LooseDot(x) => x.conditions(),
            Primitive::FixedSeg(x) => x.conditions(),
            Primitive::LoneLooseSeg(x) => x.conditions(),
            Primitive::SeqLooseSeg(x) => x.conditions(),
            Primitive::FixedBend(x) => x.conditions(),
            Primitive::LooseBend(x) => x.conditions(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GenericPrimitive<'a, W, CW, Cek, R> {
    pub index: GenericIndex<W>,
    drawing: &'a Drawing<CW, Cek, R>,
}

impl<'a, W, CW, Cek, R> GenericPrimitive<'a, W, CW, Cek, R> {
    pub fn new(index: GenericIndex<W>, drawing: &'a Drawing<CW, Cek, R>) -> Self {
        Self { index, drawing }
    }

    fn tagged_weight(&self) -> PrimitiveWeight {
        if let GenericNode::Primitive(weight) = self
            .drawing
            .geometry()
            .graph()
            .node_weight(self.index.petgraph_index())
            .unwrap()
        {
            *weight
        } else {
            unreachable!()
        }
    }
}

impl<W, CW, Cek, R> GetInterior<PrimitiveIndex> for GenericPrimitive<'_, W, CW, Cek, R> {
    fn interior(&self) -> Vec<PrimitiveIndex> {
        vec![self.tagged_weight().retag(self.index.petgraph_index())]
    }
}

impl<W, CW, Cek, R> GetDrawing for GenericPrimitive<'_, W, CW, Cek, R> {
    type CompoundWeight = CW;
    type CompoundEntryKind = Cek;
    type Rules = R;
    fn drawing(&self) -> &Drawing<CW, Cek, R> {
        self.drawing
    }
}

impl<W, CW, Cek, R> GetPetgraphIndex for GenericPrimitive<'_, W, CW, Cek, R> {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.index.petgraph_index()
    }
}

impl<'a, W: GetWidth, CW, Cek, R> GetWidth for GenericPrimitive<'a, W, CW, Cek, R>
where
    GenericPrimitive<'a, W, CW, Cek, R>: GetWeight<W>,
{
    fn width(&self) -> f64 {
        self.weight().width()
    }
}

impl<'a, W, CW, Cek, R> GetConditions<'a> for &GenericPrimitive<'a, W, CW, Cek, R>
where
    GenericPrimitive<'a, W, CW, Cek, R>: GetMaybeNet,
{
    fn conditions(self) -> Option<Conditions<'a>> {
        self.maybe_net().map(|net| Conditions {
            net,
            maybe_region: Some("A".into()),
            maybe_layer: Some("F.Cu".into()),
        })
    }
}

pub type FixedDot<'a, CW, Cek, R> = GenericPrimitive<'a, FixedDotWeight, CW, Cek, R>;
impl_fixed_primitive!(FixedDot, FixedDotWeight);

impl<CW, Cek, R> MakePrimitiveShape for FixedDot<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for FixedDot<'_, CW, Cek, R> {
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

impl<CW, Cek, R> GetFirstGear for FixedDot<'_, CW, Cek, R> {}

pub type LooseDot<'a, CW, Cek, R> = GenericPrimitive<'a, LooseDotWeight, CW, Cek, R>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<CW, Cek, R> LooseDot<'_, CW, Cek, R> {
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

impl<CW, Cek, R> MakePrimitiveShape for LooseDot<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for LooseDot<'_, CW, Cek, R> {
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

pub type FixedSeg<'a, CW, Cek, R> = GenericPrimitive<'a, FixedSegWeight, CW, Cek, R>;
impl_fixed_primitive!(FixedSeg, FixedSegWeight);

impl<CW, Cek, R> MakePrimitiveShape for FixedSeg<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for FixedSeg<'_, CW, Cek, R> {}

impl<CW, Cek, R> GetJoints<FixedDotIndex, FixedDotIndex> for FixedSeg<'_, CW, Cek, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

pub type LoneLooseSeg<'a, CW, Cek, R> = GenericPrimitive<'a, LoneLooseSegWeight, CW, Cek, R>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<CW, Cek, R> MakePrimitiveShape for LoneLooseSeg<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for LoneLooseSeg<'_, CW, Cek, R> {}

impl<CW, Cek, R> GetJoints<FixedDotIndex, FixedDotIndex> for LoneLooseSeg<'_, CW, Cek, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

pub type SeqLooseSeg<'a, CW, Cek, R> = GenericPrimitive<'a, SeqLooseSegWeight, CW, Cek, R>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<CW, Cek, R> MakePrimitiveShape for SeqLooseSeg<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for SeqLooseSeg<'_, CW, Cek, R> {}

impl<CW, Cek, R> GetJoints<DotIndex, LooseDotIndex> for SeqLooseSeg<'_, CW, Cek, R> {
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

pub type FixedBend<'a, CW, Cek, R> = GenericPrimitive<'a, FixedBendWeight, CW, Cek, R>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<CW, Cek, R> GetBendIndex for FixedBend<'_, CW, Cek, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<CW, Cek, R> MakePrimitiveShape for FixedBend<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for FixedBend<'_, CW, Cek, R> {}

impl<CW, Cek, R> GetJoints<FixedDotIndex, FixedDotIndex> for FixedBend<'_, CW, Cek, R> {
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<CW, Cek, R> GetFirstGear for FixedBend<'_, CW, Cek, R> {}
//impl<'a, R: QueryRules> GetInnerOuter for FixedBend<'a, CW, Cek, R> {}

pub type LooseBend<'a, CW, Cek, R> = GenericPrimitive<'a, LooseBendWeight, CW, Cek, R>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<CW, Cek, R> GetBendIndex for LooseBend<'_, CW, Cek, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, CW: Clone, Cek: Copy, R: AccessRules> From<LooseBend<'a, CW, Cek, R>> for BendIndex {
    fn from(bend: LooseBend<'a, CW, Cek, R>) -> BendIndex {
        bend.index.into()
    }
}

impl<CW, Cek, R> MakePrimitiveShape for LooseBend<'_, CW, Cek, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cek, R> GetLimbs for LooseBend<'_, CW, Cek, R> {}

impl<CW, Cek, R> GetOffset for LooseBend<'_, CW, Cek, R> {
    fn offset(&self) -> f64 {
        self.weight().offset()
    }
}

impl<CW, Cek, R> GetJoints<LooseDotIndex, LooseDotIndex> for LooseBend<'_, CW, Cek, R> {
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.petgraph_index()),
            LooseDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<CW, Cek, R> LooseBend<'_, CW, Cek, R> {
    pub fn inner(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }

    pub fn outer(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .outer(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }
}
