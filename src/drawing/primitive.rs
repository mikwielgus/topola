// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        gear::{GearIndex, GetPrevNextInChain},
        graph::{GetMaybeNet, PrimitiveIndex, PrimitiveWeight},
        rules::{AccessRules, Conditions, GetConditions},
        seg::{FixedSegWeight, LoneLooseSegWeight, SegIndex, SeqLooseSegIndex, SeqLooseSegWeight},
        Drawing,
    },
    geometry::{primitive::PrimitiveShape, GenericNode, GetLayer, GetOffset, GetWidth, Retag},
    graph::{GenericIndex, GetPetgraphIndex},
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
    F: GetPetgraphIndex,
    S: GetJoints<F = F>,
    <S as GetJoints>::T: GetPetgraphIndex + Into<F>,
{
    type J = F;
    fn other_joint(&self, end: F) -> F {
        let joints = self.joints();
        if joints.0.petgraph_index() != end.petgraph_index() {
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

pub trait GetLowestGears: GetDrawing + GetPetgraphIndex {
    // TODO: Make it return an iterator instead of a vec.
    fn lowest_gears(&self) -> Vec<LooseBendIndex> {
        self.drawing()
            .geometry()
            .all_rails(self.petgraph_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
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
        impl<CW, Cel, R> GetWeight for $primitive_struct<'_, CW, Cel, R> {
            type Weight = $weight_struct;
            fn weight(&self) -> $weight_struct {
                if let PrimitiveWeight::$primitive_struct(weight) = self.tagged_weight() {
                    weight
                } else {
                    unreachable!()
                }
            }
        }

        impl<CW, Cel, R> GetLayer for $primitive_struct<'_, CW, Cel, R> {
            fn layer(&self) -> usize {
                self.weight().layer()
            }
        }

        impl<CW, Cel, R> GetMaybeNet for $primitive_struct<'_, CW, Cel, R> {
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
pub enum Primitive<'a, CW, Cel, R> {
    FixedDot(FixedDot<'a, CW, Cel, R>),
    LooseDot(LooseDot<'a, CW, Cel, R>),
    FixedSeg(FixedSeg<'a, CW, Cel, R>),
    LoneLooseSeg(LoneLooseSeg<'a, CW, Cel, R>),
    SeqLooseSeg(SeqLooseSeg<'a, CW, Cel, R>),
    FixedBend(FixedBend<'a, CW, Cel, R>),
    LooseBend(LooseBend<'a, CW, Cel, R>),
}

impl<'a, CW, Cel, R: AccessRules> GetConditions<'a> for &Primitive<'a, CW, Cel, R> {
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
            .node_weight(self.index.petgraph_index())
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
        vec![self.tagged_weight().retag(self.index.petgraph_index())]
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

impl<W, CW, Cel, R> GetPetgraphIndex for GenericPrimitive<'_, W, CW, Cel, R> {
    fn petgraph_index(&self) -> NodeIndex<usize> {
        self.index.petgraph_index()
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

pub type FixedDot<'a, CW, Cel, R> = GenericPrimitive<'a, FixedDotWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedDot, FixedDotWeight);

impl<CW, Cel, R> MakePrimitiveShape for FixedDot<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedDot<'_, CW, Cel, R> {
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

impl<CW, Cel, R> GetLowestGears for FixedDot<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOuterGears for FixedDot<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.lowest_gears()
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules> GetPrevNextInChain for FixedDot<'_, CW, Cel, R> {
    fn next_in_chain(&self, maybe_prev: Option<GearIndex>) -> Option<GearIndex> {
        self.drawing
            .overlapees(self.index.into())
            .find_map(|infringement| {
                let PrimitiveIndex::FixedDot(intersectee) = infringement.1 else {
                    return None;
                };

                if let Some(prev) = maybe_prev {
                    (infringement.1 == prev.into()).then_some(intersectee.into())
                } else {
                    Some(intersectee.into())
                }
            })
    }
}

impl<CW, Cel, R> WalkOutwards for FixedDot<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.lowest_gears().into_iter())
    }
}

pub type LooseDot<'a, CW, Cel, R> = GenericPrimitive<'a, LooseDotWeight, CW, Cel, R>;
impl_loose_primitive!(LooseDot, LooseDotWeight);

impl<CW, Cel, R> LooseDot<'_, CW, Cel, R> {
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

impl<CW, Cel, R> MakePrimitiveShape for LooseDot<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().dot_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LooseDot<'_, CW, Cel, R> {
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

pub type FixedSeg<'a, CW, Cel, R> = GenericPrimitive<'a, FixedSegWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedSeg, FixedSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for FixedSeg<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedSeg<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for FixedSeg<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

pub type LoneLooseSeg<'a, CW, Cel, R> = GenericPrimitive<'a, LoneLooseSegWeight, CW, Cel, R>;
impl_loose_primitive!(LoneLooseSeg, LoneLooseSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for LoneLooseSeg<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LoneLooseSeg<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for LoneLooseSeg<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().seg_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

pub type SeqLooseSeg<'a, CW, Cel, R> = GenericPrimitive<'a, SeqLooseSegWeight, CW, Cel, R>;
impl_loose_primitive!(SeqLooseSeg, SeqLooseSegWeight);

impl<CW, Cel, R> MakePrimitiveShape for SeqLooseSeg<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().seg_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for SeqLooseSeg<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for SeqLooseSeg<'_, CW, Cel, R> {
    type F = DotIndex;
    type T = LooseDotIndex;
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

pub type FixedBend<'a, CW, Cel, R> = GenericPrimitive<'a, FixedBendWeight, CW, Cel, R>;
impl_fixed_primitive!(FixedBend, FixedBendWeight);

impl<CW, Cel, R> GetBendIndex for FixedBend<'_, CW, Cel, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<CW, Cel, R> MakePrimitiveShape for FixedBend<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for FixedBend<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetJoints for FixedBend<'_, CW, Cel, R> {
    type F = FixedDotIndex;
    type T = FixedDotIndex;
    fn joints(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            FixedDotIndex::new(from.petgraph_index()),
            FixedDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<CW, Cel, R> GetLowestGears for FixedBend<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOuterGears for FixedBend<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.lowest_gears()
    }
}

impl<CW, Cel, R> GetPrevNextInChain for FixedBend<'_, CW, Cel, R> {
    fn next_in_chain(&self, _maybe_prev: Option<GearIndex>) -> Option<GearIndex> {
        None
    }
}

impl<CW, Cel, R> WalkOutwards for FixedBend<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.lowest_gears().into_iter())
    }
}

pub type LooseBend<'a, CW, Cel, R> = GenericPrimitive<'a, LooseBendWeight, CW, Cel, R>;
impl_loose_primitive!(LooseBend, LooseBendWeight);

impl<CW, Cel, R> GetBendIndex for LooseBend<'_, CW, Cel, R> {
    fn bend_index(&self) -> BendIndex {
        self.index.into()
    }
}

impl<'a, CW: Clone, Cel: Copy, R: AccessRules> From<LooseBend<'a, CW, Cel, R>> for BendIndex {
    fn from(bend: LooseBend<'a, CW, Cel, R>) -> BendIndex {
        bend.index.into()
    }
}

impl<CW, Cel, R> MakePrimitiveShape for LooseBend<'_, CW, Cel, R> {
    fn shape(&self) -> PrimitiveShape {
        self.drawing.geometry().bend_shape(self.index.into())
    }
}

impl<CW, Cel, R> GetLimbs for LooseBend<'_, CW, Cel, R> {}

impl<CW, Cel, R> GetOffset for LooseBend<'_, CW, Cel, R> {
    fn offset(&self) -> f64 {
        self.weight().offset()
    }
}

impl<CW, Cel, R> GetJoints for LooseBend<'_, CW, Cel, R> {
    type F = LooseDotIndex;
    type T = LooseDotIndex;
    fn joints(&self) -> (LooseDotIndex, LooseDotIndex) {
        let (from, to) = self.drawing.geometry().bend_joints(self.index.into());
        (
            LooseDotIndex::new(from.petgraph_index()),
            LooseDotIndex::new(to.petgraph_index()),
        )
    }
}

impl<CW, Cel, R> GetOuterGears for LooseBend<'_, CW, Cel, R> {
    fn outer_gears(&self) -> Vec<LooseBendIndex> {
        self.outers().collect()
    }
}

impl<CW, Cel, R> GetPrevNextInChain for LooseBend<'_, CW, Cel, R> {
    fn next_in_chain(&self, _maybe_prev: Option<GearIndex>) -> Option<GearIndex> {
        None
    }
}

impl<CW, Cel, R> WalkOutwards for LooseBend<'_, CW, Cel, R> {
    fn outwards(&self) -> DrawingOutwardWalker {
        DrawingOutwardWalker::new(self.outers())
    }
}

impl<CW, Cel, R> LooseBend<'_, CW, Cel, R> {
    pub fn inner(&self) -> Option<LooseBendIndex> {
        self.drawing()
            .geometry()
            .inner(self.bend_index())
            .map(|ni| LooseBendIndex::new(ni.petgraph_index()))
    }

    pub fn outers(&self) -> impl Iterator<Item = LooseBendIndex> + '_ {
        self.drawing()
            .geometry()
            .outers(self.bend_index())
            .map(|node| LooseBendIndex::new(node.petgraph_index()))
    }
}
