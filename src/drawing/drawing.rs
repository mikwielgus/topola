// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::{debug_ensures, debug_invariant};
use derive_getters::Getters;
use geo::Point;

use core::fmt;
use rstar::{RTree, AABB};
use thiserror::Error;

use crate::geometry::{
    edit::{ApplyGeometryEdit, GeometryEdit},
    primitive::{AccessPrimitiveShape, PrimitiveShape},
    recording_with_rtree::RecordingGeometryWithRtree,
    with_rtree::BboxedIndex,
    AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GeometryLabel,
    GetLayer, GetOffset, GetSetPos, GetWidth,
};
use crate::graph::{GenericIndex, GetPetgraphIndex};
use crate::math::NoTangents;
use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, BendWeight, FixedBendIndex, LooseBendIndex, LooseBendWeight},
        cane::Cane,
        collect::Collect,
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        gear::{GearIndex, GetNextGear},
        graph::{GetMaybeNet, IsInLayer, MakePrimitive, PrimitiveIndex, PrimitiveWeight},
        guide::Guide,
        loose::{GetPrevNextLoose, Loose, LooseIndex},
        primitive::{
            GenericPrimitive, GetCore, GetInnerOuter, GetJoints, GetLimbs, GetOtherJoint,
            MakePrimitiveShape,
        },
        rules::{AccessRules, GetConditions},
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
            SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
        },
    },
    graph::MakeRef,
};

#[derive(Clone, Copy, Error)]
pub enum DrawingException {
    #[error(transparent)]
    NoTangents(#[from] NoTangents),
    #[error(transparent)]
    Infringement(#[from] Infringement),
    #[error(transparent)]
    Collision(#[from] Collision),
    #[error(transparent)]
    AlreadyConnected(#[from] AlreadyConnected),
}

impl fmt::Debug for DrawingException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTangents(this) => fmt::Debug::fmt(this, f),
            Self::Infringement(this) => fmt::Debug::fmt(this, f),
            Self::Collision(this) => fmt::Debug::fmt(this, f),
            Self::AlreadyConnected(this) => fmt::Debug::fmt(this, f),
        }
    }
}

// TODO add real error messages + these should eventually use Display
#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} infringes on {1:?}")]
pub struct Infringement(pub PrimitiveShape, pub PrimitiveIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} collides with {1:?}")]
pub struct Collision(pub PrimitiveShape, pub PrimitiveIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{1:?} is already connected to net {0}")]
pub struct AlreadyConnected(pub usize, pub PrimitiveIndex);

pub type DrawingEdit<CW> = GeometryEdit<
    PrimitiveWeight,
    DotWeight,
    SegWeight,
    BendWeight,
    CW,
    PrimitiveIndex,
    DotIndex,
    SegIndex,
    BendIndex,
>;

#[derive(Debug, Getters)]
pub struct Drawing<CW, R> {
    recording_geometry_with_rtree: RecordingGeometryWithRtree<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    >,
    rules: R,
}

#[debug_invariant(self.test_if_looses_dont_infringe_each_other())]
impl<CW: Copy, R: AccessRules> Drawing<CW, R> {
    pub fn new(rules: R, layer_count: usize) -> Self {
        Self {
            recording_geometry_with_rtree: RecordingGeometryWithRtree::new(layer_count),
            rules,
        }
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        match band {
            BandTermsegIndex::Straight(seg) => {
                self.recording_geometry_with_rtree
                    .remove_seg(recorder, seg.into());
            }
            BandTermsegIndex::Bended(first_loose_seg) => {
                let mut dots = vec![];
                let mut segs = vec![];
                let mut bends = vec![];
                let mut outers = vec![];

                let mut maybe_loose = Some(first_loose_seg.into());
                let mut prev = None;

                while let Some(loose) = maybe_loose {
                    match loose {
                        LooseIndex::Dot(dot) => {
                            dots.push(dot);
                        }
                        LooseIndex::LoneSeg(seg) => {
                            self.recording_geometry_with_rtree
                                .remove_seg(recorder, seg.into());
                            break;
                        }
                        LooseIndex::SeqSeg(seg) => {
                            segs.push(seg);
                        }
                        LooseIndex::Bend(bend) => {
                            bends.push(bend);

                            if let Some(outer) = self.primitive(bend).outer() {
                                outers.push(outer);
                                self.reattach_bend(recorder, outer, self.primitive(bend).inner());
                            }
                        }
                    }

                    let prev_prev = prev;
                    prev = maybe_loose;
                    maybe_loose = self.loose(loose).next_loose(prev_prev);
                }

                for bend in bends {
                    self.recording_geometry_with_rtree
                        .remove_bend(recorder, bend.into());
                }

                for seg in segs {
                    self.recording_geometry_with_rtree
                        .remove_seg(recorder, seg.into());
                }

                // We must remove the dots only after the segs and bends because we need dots to calculate
                // the shapes, which we first need unchanged to remove the segs and bends from the R-tree.

                for dot in dots {
                    self.recording_geometry_with_rtree
                        .remove_dot(recorder, dot.into());
                }

                for outer in outers {
                    self.update_this_and_outward_bows(recorder, outer)?;
                }
            }
        }

        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_dot(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        weight: FixedDotWeight,
    ) -> Result<FixedDotIndex, Infringement> {
        self.add_dot_with_infringables(recorder, weight, Some(&[]))
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() - 1))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn remove_fixed_dot(&mut self, recorder: &mut DrawingEdit<CW>, dot: FixedDotIndex) {
        self.recording_geometry_with_rtree
            .remove_dot(recorder, dot.into());
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        weight: FixedDotWeight,
    ) -> FixedDotIndex {
        self.add_dot_infringably(recorder, weight)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    fn add_dot_with_infringables<W: AccessDotWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        weight: W,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let dot = self.add_dot_infringably(recorder, weight);
        self.fail_and_remove_if_infringes_except(recorder, dot.into(), infringables)?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.add_seg_with_infringables(recorder, from.into(), to.into(), weight, Some(&[]))
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 2))]
    pub fn add_fixed_seg_infringably(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> FixedSegIndex {
        self.add_seg_infringably(recorder, from.into(), to.into(), weight)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        let seg =
            self.add_seg_with_infringables(recorder, from.into(), to.into(), weight, Some(&[]))?;
        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
        let seg = self.add_seg_with_infringables(recorder, from, to.into(), weight, Some(&[]))?;
        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() >= old(self.recording_geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_seg_with_infringables<W: AccessSegWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        to: DotIndex,
        weight: W,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let seg = self.add_seg_infringably(recorder, from, to, weight);
        self.fail_and_remove_if_infringes_except(recorder, seg.into(), infringables)?;

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 3)
        || self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_loose_bend_with_infringables(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: GearIndex,
        weight: LooseBendWeight,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<LooseBendIndex, DrawingException> {
        // It makes no sense to wrap something around or under one of its connectables.
        //
        if let Some(net) = weight.maybe_net() {
            if let Some(around_net) = around.primitive(self).maybe_net() {
                if net == around_net {
                    return Err(AlreadyConnected(net, around.into()).into());
                }
            }
            //
            if let Some(next_gear) = around.ref_(self).next_gear() {
                if let Some(next_gear_net) = next_gear.primitive(self).maybe_net() {
                    if net == next_gear_net {
                        return Err(AlreadyConnected(net, next_gear.into()).into());
                    }
                }
            }
        }

        match around {
            GearIndex::FixedDot(core) => self
                .add_core_bend_with_infringables(
                    recorder,
                    from.into(),
                    to.into(),
                    core,
                    weight,
                    infringables,
                )
                .map_err(Into::into),
            GearIndex::FixedBend(around) => self
                .add_outer_bend_with_infringables(
                    recorder,
                    from,
                    to,
                    around.into(),
                    weight,
                    infringables,
                )
                .map_err(Into::into),
            GearIndex::LooseBend(around) => self
                .add_outer_bend_with_infringables(
                    recorder,
                    from,
                    to,
                    around.into(),
                    weight,
                    infringables,
                )
                .map_err(Into::into),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_core_bend_with_infringables<W: AccessBendWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        to: DotIndex,
        core: FixedDotIndex,
        weight: W,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let bend =
            self.recording_geometry_with_rtree
                .add_bend(recorder, from, to, core.into(), weight);

        self.fail_and_remove_if_infringes_except(recorder, bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_outer_bend_with_infringables(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: LooseDotIndex,
        to: LooseDotIndex,
        inner: BendIndex,
        weight: LooseBendWeight,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<GenericIndex<LooseBendWeight>, Infringement> {
        let core = *self
            .recording_geometry_with_rtree
            .graph()
            .neighbors(inner.petgraph_index())
            .filter(|ni| {
                matches!(
                    self.recording_geometry_with_rtree
                        .graph()
                        .edge_weight(
                            self.recording_geometry_with_rtree
                                .graph()
                                .find_edge(inner.petgraph_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(FixedDotIndex::new)
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = self.recording_geometry_with_rtree.add_bend(
            recorder,
            from.into(),
            to.into(),
            core.into(),
            weight,
        );
        self.recording_geometry_with_rtree
            .reattach_bend(recorder, bend.into(), Some(inner));

        self.fail_and_remove_if_infringes_except(recorder, bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn flip_bend(&mut self, recorder: &mut DrawingEdit<CW>, bend: FixedBendIndex) {
        self.recording_geometry_with_rtree
            .flip_bend(recorder, bend.into());
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count())
        || self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() - 1)
        || self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 1))]
    fn reattach_bend(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        bend: LooseBendIndex,
        maybe_new_inner: Option<LooseBendIndex>,
    ) {
        self.recording_geometry_with_rtree.reattach_bend(
            recorder,
            bend.into(),
            maybe_new_inner.map(Into::into),
        );
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() >= old(self.recording_geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn insert_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Cane, DrawingException> {
        let maybe_next_gear = around.ref_(self).next_gear();
        let cane = self.add_cane_with_infringables(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
            Some(&[]),
        )?;

        if let Some(next_gear) = maybe_next_gear {
            self.reattach_bend(recorder, next_gear, Some(cane.bend));
        }

        if let Some(outer) = self.primitive(cane.bend).outer() {
            self.update_this_and_outward_bows(recorder, outer)
                .inspect_err(|_| {
                    let joint = self.primitive(cane.bend).other_joint(cane.dot);
                    self.remove_cane(recorder, &cane, joint);
                })?;
        }

        // Segs must not cross.
        if let Some(collision) = self.detect_collision(cane.seg.into()) {
            let joint = self.primitive(cane.bend).other_joint(cane.dot);
            self.remove_cane(recorder, &cane, joint);
            Err(collision.into())
        } else {
            Ok(cane)
        }
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn update_this_and_outward_bows(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        around: LooseBendIndex,
    ) -> Result<(), DrawingException> {
        // FIXME: Fail gracefully on infringement.
        let mut maybe_rail = Some(around);

        while let Some(rail) = maybe_rail {
            let rail_primitive = self.primitive(rail);
            let joints = rail_primitive.joints();

            let from_head = self.rear_head(joints.1);
            let to_head = self.rear_head(joints.0);

            if let Some(inner) = rail_primitive.inner() {
                let from = self
                    .head_around_bend_segment(
                        &from_head,
                        inner.into(),
                        true,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let to = self
                    .head_around_bend_segment(
                        &to_head,
                        inner.into(),
                        false,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let offset = self.head_around_bend_offset(
                    &from_head,
                    inner.into(),
                    self.primitive(rail).width(),
                );

                self.move_dot_with_infringables(
                    recorder,
                    joints.0.into(),
                    from,
                    Some(&self.bend_outer_bows(rail)),
                )?;
                self.move_dot_with_infringables(
                    recorder,
                    joints.1.into(),
                    to,
                    Some(&self.bend_outer_bows(rail)),
                )?;

                self.shift_bend_with_infringables(
                    recorder,
                    rail.into(),
                    offset,
                    Some(&self.bend_outer_bows(rail)),
                )?;

                // Update offsets in case the rule conditions changed.
            } else {
                let core = rail_primitive.core();
                let from = self
                    .head_around_dot_segment(
                        &from_head,
                        core.into(),
                        true,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let to = self
                    .head_around_dot_segment(
                        &to_head,
                        core.into(),
                        false,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let offset = self.head_around_dot_offset(
                    &from_head,
                    core.into(),
                    self.primitive(rail).width(),
                );

                self.move_dot_with_infringables(
                    recorder,
                    joints.0.into(),
                    from,
                    Some(&self.bend_outer_bows(rail)),
                )?;
                self.move_dot_with_infringables(
                    recorder,
                    joints.1.into(),
                    to,
                    Some(&self.bend_outer_bows(rail)),
                )?;

                self.shift_bend_with_infringables(
                    recorder,
                    rail.into(),
                    offset,
                    Some(&self.bend_outer_bows(rail)),
                )?;
            }

            maybe_rail = self.primitive(rail).outer();
        }

        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() >= old(self.recording_geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn add_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Cane, DrawingException> {
        self.add_cane_with_infringables(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
            Some(&[]),
        )
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() >= old(self.recording_geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_cane_with_infringables(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<Cane, DrawingException> {
        let seg_to = self.add_dot_with_infringables(recorder, dot_weight, infringables)?;
        let seg = self
            .add_seg_with_infringables(recorder, from, seg_to.into(), seg_weight, infringables)
            .inspect_err(|_| {
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, seg_to.into());
            })?;

        let to = self
            .add_dot_with_infringables(recorder, dot_weight, infringables)
            .inspect_err(|_| {
                self.recording_geometry_with_rtree
                    .remove_seg(recorder, seg.into());
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, seg_to.into());
            })?;

        let (bend_from, bend_to) = if cw { (to, seg_to) } else { (seg_to, to) };

        let bend = self
            .add_loose_bend_with_infringables(
                recorder,
                bend_from,
                bend_to,
                around,
                bend_weight,
                infringables,
            )
            .inspect_err(|_| {
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, to.into());
                self.recording_geometry_with_rtree
                    .remove_seg(recorder, seg.into());
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, seg_to.into());
            })?;

        Ok(Cane {
            seg,
            dot: seg_to,
            bend,
        })
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() - 4))]
    pub fn remove_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        cane: &Cane,
        face: LooseDotIndex,
    ) {
        let maybe_outer = self.primitive(cane.bend).outer();

        // Removing a loose bend affects its outer bends.
        if let Some(outer) = maybe_outer {
            self.reattach_bend(recorder, outer, self.primitive(cane.bend).inner());
        }

        self.recording_geometry_with_rtree
            .remove_bend(recorder, cane.bend.into());
        self.recording_geometry_with_rtree
            .remove_seg(recorder, cane.seg.into());

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we first need unchanged to remove the segs and bends from the R-tree.

        self.recording_geometry_with_rtree
            .remove_dot(recorder, face.into());
        self.recording_geometry_with_rtree
            .remove_dot(recorder, cane.dot.into());

        if let Some(outer) = maybe_outer {
            self.update_this_and_outward_bows(recorder, outer).unwrap(); // Must never fail.
        }
    }

    pub fn cane(&self, dot: LooseDotIndex) -> Cane {
        Cane::from_dot(dot, self)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn move_dot(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        dot: DotIndex,
        to: Point,
    ) -> Result<(), Infringement> {
        match dot {
            DotIndex::Fixed(..) => self.move_dot_with_infringables(recorder, dot, to, Some(&[])),
            DotIndex::Loose(..) => self.move_dot_with_infringables(recorder, dot, to, Some(&[])),
        }
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn move_dot_with_infringables(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        dot: DotIndex,
        to: Point,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<(), Infringement> {
        let old_pos = self
            .recording_geometry_with_rtree
            .geometry()
            .dot_weight(dot)
            .pos();
        self.recording_geometry_with_rtree
            .move_dot(recorder, dot, to);

        for limb in dot.primitive(self).limbs() {
            if let Some(infringement) = self.detect_infringement_except(limb, infringables) {
                // Restore original state.
                self.recording_geometry_with_rtree
                    .move_dot(recorder, dot, old_pos);
                return Err(infringement);
            }
        }

        if let Some(infringement) = self.detect_infringement_except(dot.into(), infringables) {
            // Restore original state.
            self.recording_geometry_with_rtree
                .move_dot(recorder, dot, old_pos);
            return Err(infringement);
        }

        Ok(())
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn shift_bend_with_infringables(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        bend: BendIndex,
        offset: f64,
        infringables: Option<&[PrimitiveIndex]>,
    ) -> Result<(), Infringement> {
        let old_offset = self
            .recording_geometry_with_rtree
            .geometry()
            .bend_weight(bend)
            .offset();
        self.recording_geometry_with_rtree
            .shift_bend(recorder, bend, offset);

        if let Some(infringement) = self.detect_infringement_except(bend.into(), infringables) {
            // Restore original state.
            self.recording_geometry_with_rtree
                .shift_bend(recorder, bend, old_offset);
            return Err(infringement);
        }

        Ok(())
    }

    fn detect_collision(&self, node: PrimitiveIndex) -> Option<Collision> {
        let shape = node.primitive(self).shape();

        self.recording_geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&shape.full_height_envelope_3d(0.0, 2))
            .filter_map(|wrapper| {
                if let GenericNode::Primitive(primitive_node) = wrapper.data {
                    Some(primitive_node)
                } else {
                    None
                }
            })
            .filter(|primitive_node| !self.are_connectable(node, *primitive_node))
            .find(|primitive_node| shape.intersects(&primitive_node.primitive(self).shape()))
            .map(|collidee| Collision(shape, collidee))
    }
}

impl<CW: Copy, R: AccessRules> Drawing<CW, R> {
    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    fn add_dot_infringably<W: AccessDotWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        self.recording_geometry_with_rtree.add_dot(recorder, weight)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count() + 2))]
    fn add_seg_infringably<W: AccessSegWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        from: DotIndex,
        to: DotIndex,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PrimitiveIndex>,
    {
        self.recording_geometry_with_rtree
            .add_seg(recorder, from, to, weight)
    }

    pub fn add_compound(&mut self, recorder: &mut DrawingEdit<CW>, weight: CW) -> GenericIndex<CW> {
        self.recording_geometry_with_rtree
            .add_compound(recorder, weight)
    }

    pub fn remove_compound(&mut self, recorder: &mut DrawingEdit<CW>, compound: GenericIndex<CW>) {
        self.recording_geometry_with_rtree
            .remove_compound(recorder, compound);
    }

    pub fn add_to_compound<W>(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        primitive: GenericIndex<W>,
        compound: GenericIndex<CW>,
    ) {
        self.recording_geometry_with_rtree
            .add_to_compound(recorder, primitive, compound);
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count() - 1))]
    fn fail_and_remove_if_infringes_except(
        &mut self,
        recorder: &mut DrawingEdit<CW>,
        node: PrimitiveIndex,
        maybe_except: Option<&[PrimitiveIndex]>,
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.detect_infringement_except(node, maybe_except) {
            if let Ok(dot) = node.try_into() {
                self.recording_geometry_with_rtree.remove_dot(recorder, dot);
            } else if let Ok(seg) = node.try_into() {
                self.recording_geometry_with_rtree.remove_seg(recorder, seg);
            } else if let Ok(bend) = node.try_into() {
                self.recording_geometry_with_rtree
                    .remove_bend(recorder, bend);
            }
            return Err(infringement);
        }
        Ok(())
    }

    fn detect_infringement_except(
        &self,
        node: PrimitiveIndex,
        maybe_except: Option<&[PrimitiveIndex]>,
    ) -> Option<Infringement> {
        self.find_infringement(
            node,
            self.locate_possible_infringers(node)
                .filter_map(|n| {
                    if let GenericNode::Primitive(primitive_node) = n {
                        Some(primitive_node)
                    } else {
                        None
                    }
                })
                .filter(|primitive_node| {
                    maybe_except.is_some_and(|except| !except.contains(primitive_node))
                }),
        )
    }

    fn locate_possible_infringers(
        &self,
        node: PrimitiveIndex,
    ) -> impl Iterator<Item = GenericNode<PrimitiveIndex, GenericIndex<CW>>> + '_ {
        let limiting_shape = node.primitive(self).shape().inflate(
            node.primitive(self)
                .maybe_net()
                .map(|net| self.rules.largest_clearance(Some(net)))
                .unwrap_or(0.0),
        );

        self.recording_geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(
                &limiting_shape.envelope_3d(0.0, node.primitive(self).layer()),
            )
            .map(|wrapper| wrapper.data)
    }

    fn find_infringement(
        &self,
        node: PrimitiveIndex,
        it: impl Iterator<Item = PrimitiveIndex>,
    ) -> Option<Infringement> {
        let mut inflated_shape = node.primitive(self).shape(); // Unused temporary value just for initialization.
        let conditions = node.primitive(self).conditions();

        it.filter(|primitive_node| !self.are_connectable(node, *primitive_node))
            .find_map(|primitive_node| {
                let infringee_conditions = primitive_node.primitive(self).conditions();

                let epsilon = 1.0;
                inflated_shape = node.primitive(self).shape().inflate(
                    (self.rules.clearance(&conditions, &infringee_conditions) - epsilon)
                        .clamp(0.0, f64::INFINITY),
                );

                inflated_shape
                    .intersects(&primitive_node.primitive(self).shape())
                    .then_some(Infringement(inflated_shape, primitive_node))
            })
    }

    pub fn primitive_nodes(&self) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.recording_geometry_with_rtree
            .rtree()
            .iter()
            .filter_map(|wrapper| {
                if let GenericNode::Primitive(primitive_node) = wrapper.data {
                    Some(primitive_node)
                } else {
                    None
                }
            })
    }

    pub fn layer_primitive_nodes(&self, layer: usize) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.recording_geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .filter_map(|wrapper| {
                if let GenericNode::Primitive(primitive_node) = wrapper.data {
                    Some(primitive_node)
                } else {
                    None
                }
            })
    }

    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> CW {
        self.recording_geometry_with_rtree.compound_weight(compound)
    }

    pub fn compounds<'a, W: 'a>(
        &'a self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = GenericIndex<CW>> + 'a {
        self.recording_geometry_with_rtree.compounds(node)
    }

    pub fn is_node_in_layer(
        &self,
        index: GenericNode<PrimitiveIndex, GenericIndex<CW>>,
        active_layer: usize,
    ) -> bool
    where
        CW: super::graph::IsInLayer,
    {
        match index {
            GenericNode::Primitive(primitive) => primitive.primitive(self).layer() == active_layer,
            GenericNode::Compound(compound) => {
                self.compound_weight(compound).is_in_layer(active_layer)
            }
        }
    }

    pub fn is_node_in_any_layer_of(
        &self,
        index: GenericNode<PrimitiveIndex, GenericIndex<CW>>,
        layers: &[bool],
    ) -> bool
    where
        CW: IsInLayer,
    {
        match index {
            GenericNode::Primitive(primitive) => {
                primitive.primitive(self).is_in_any_layer_of(layers)
            }
            GenericNode::Compound(compound) => {
                self.compound_weight(compound).is_in_any_layer_of(layers)
            }
        }
    }

    fn are_connectable(&self, node1: PrimitiveIndex, node2: PrimitiveIndex) -> bool {
        if let (Some(node1_net_id), Some(node2_net_id)) = (
            node1.primitive(self).maybe_net(),
            node2.primitive(self).maybe_net(),
        ) {
            node1_net_id == node2_net_id
        } else {
            true
        }
    }

    pub fn geometry(
        &self,
    ) -> &Geometry<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > {
        self.recording_geometry_with_rtree.geometry()
    }

    pub fn rtree(&self) -> &RTree<BboxedIndex<GenericNode<PrimitiveIndex, GenericIndex<CW>>>> {
        self.recording_geometry_with_rtree.rtree()
    }

    #[debug_ensures(self.recording_geometry_with_rtree.graph().node_count() == old(self.recording_geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.recording_geometry_with_rtree.graph().edge_count() == old(self.recording_geometry_with_rtree.graph().edge_count()))]
    pub fn rules_mut(&mut self) -> &mut R {
        &mut self.rules
    }

    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W, CW, R> {
        GenericPrimitive::new(index, self)
    }

    pub fn loose(&self, index: LooseIndex) -> Loose<CW, R> {
        Loose::new(index, self)
    }

    pub fn layer_count(&self) -> usize {
        self.recording_geometry_with_rtree.layer_count()
    }

    pub fn node_count(&self) -> usize {
        self.recording_geometry_with_rtree.graph().node_count()
    }

    fn test_if_looses_dont_infringe_each_other(&self) -> bool {
        !self
            .primitive_nodes()
            .filter(|node| {
                matches!(
                    node,
                    PrimitiveIndex::LooseDot(..)
                        | PrimitiveIndex::LoneLooseSeg(..)
                        | PrimitiveIndex::SeqLooseSeg(..)
                        | PrimitiveIndex::LooseBend(..)
                )
            })
            .any(|node| {
                self.find_infringement(
                    node,
                    self.locate_possible_infringers(node)
                        .filter_map(|n| {
                            if let GenericNode::Primitive(primitive_node) = n {
                                Some(primitive_node)
                            } else {
                                None
                            }
                        })
                        .filter(|primitive_node| {
                            matches!(
                                primitive_node,
                                PrimitiveIndex::LooseDot(..)
                                    | PrimitiveIndex::LoneLooseSeg(..)
                                    | PrimitiveIndex::SeqLooseSeg(..)
                                    | PrimitiveIndex::LooseBend(..)
                            )
                        }),
                )
                .is_some()
            })
    }
}

impl<CW: Copy, R: AccessRules>
    ApplyGeometryEdit<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Drawing<CW, R>
{
    fn apply(&mut self, edit: &DrawingEdit<CW>) {
        self.recording_geometry_with_rtree.apply(edit);
    }
}
