// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::{debug_ensures, debug_invariant};
use derive_getters::Getters;
use geo::{Point, Polygon};
use petgraph::visit::Walker;

use core::fmt;
use rstar::{RTree, AABB};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        bend::{BendIndex, BendWeight, FixedBendIndex, LooseBendIndex, LooseBendWeight},
        cane::Cane,
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
        gear::GearIndex,
        graph::{GetMaybeNet, IsInLayer, MakePrimitiveRef, PrimitiveIndex, PrimitiveWeight},
        loose::{GetPrevNextLoose, Loose, LooseIndex},
        primitive::{
            GenericPrimitive, GetCore, GetJoints, GetLimbs, GetOtherJoint, MakePrimitiveShape,
        },
        rules::AccessRules,
        seg::{
            FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
            SegWeight, SeqLooseSegIndex, SeqLooseSegWeight,
        },
    },
    geometry::{
        edit::{ApplyGeometryEdit, GeometryEdit},
        primitive::PrimitiveShape,
        recording_with_rtree::RecordingGeometryWithRtree,
        with_rtree::BboxedIndex,
        AccessBendWeight, AccessDotWeight, AccessSegWeight, GenericNode, Geometry, GeometryLabel,
        GetLayer, GetOffset, GetSetPos, GetWidth,
    },
    graph::{GenericIndex, GetIndex, MakeRef},
    math::{NoBitangents, RotationSense},
};

use super::gear::{GetOuterGears, WalkOutwards};

#[derive(Clone, Copy, Error)]
pub enum DrawingException {
    #[error(transparent)]
    NoTangents(#[from] NoBitangents),
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

impl DrawingException {
    pub fn maybe_ghost_and_obstacle(&self) -> Option<(&PrimitiveShape, PrimitiveIndex)> {
        match self {
            Self::NoTangents(_) => None,
            Self::Infringement(Infringement(ghost, obstacle)) => Some((ghost, *obstacle)),
            Self::Collision(Collision(ghost, obstacle)) => Some((ghost, *obstacle)),
            Self::AlreadyConnected(_) => None,
        }
    }
}

// TODO add real error messages + these should eventually use Display

/// An infringement occurs when there is an intersection of shapes formed by
/// inflating the shapes of two non-connectable primitives by their clearances.
///
/// Infringement exceptions are never raised for two same-net primitives, since
/// having the same net implies being connectable.
#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} infringes on {1:?}")]
pub struct Infringement(pub PrimitiveShape, pub PrimitiveIndex);

/// A collision is a special case of infringement where the uninflated shapes of
/// two primitives themselves intersect. In other words, collision detection is
/// a specialized infringement detection where clearances are set to 0.
///
/// Collisions are raised in different situations than infringements, typically
/// when the usual infringement detection could not be performed to avoid
/// false positives. Hence, the exception handling code should handle both
/// infringements and collisions similarly and should not assume that collisions
/// will be subsumed into infringements.
///
/// Unlike infringements, collision exceptions can be raised for two same-net
/// primitives.
///
/// For instance, a collision exception is raised when the currently routed band
/// forms a self-intersecting loop. An infringement exception cannot be raised
/// in such case because the primitives of a single band all belong to the same
/// net, making them connectable and thus uninfringable.
#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} collides with {1:?}")]
pub struct Collision(pub PrimitiveShape, pub PrimitiveIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{1:?} is already connected to net {0}")]
pub struct AlreadyConnected(pub usize, pub PrimitiveIndex);

pub type DrawingEdit<CW, Cel> = GeometryEdit<
    DotWeight,
    SegWeight,
    BendWeight,
    CW,
    Cel,
    PrimitiveIndex,
    DotIndex,
    SegIndex,
    BendIndex,
>;

#[derive(Clone, Debug, Getters)]
pub struct Drawing<CW, Cel, R> {
    recording_geometry_with_rtree: RecordingGeometryWithRtree<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        Cel,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    >,
    rules: R,

    boundary: Polygon,
    place_boundary: Polygon,
}

impl<CW, Cel, R> Drawing<CW, Cel, R> {
    pub fn geometry(
        &self,
    ) -> &Geometry<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        Cel,
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

    pub fn rules_mut(&mut self) -> &mut R {
        &mut self.rules
    }

    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<'_, W, CW, Cel, R> {
        GenericPrimitive::new(index, self)
    }

    pub fn loose(&self, index: LooseIndex) -> Loose<'_, CW, Cel, R> {
        Loose::new(index, self)
    }

    pub fn layer_count(&self) -> usize {
        self.recording_geometry_with_rtree.layer_count()
    }

    pub fn node_count(&self) -> usize {
        self.recording_geometry_with_rtree.node_count()
    }
}

impl<CW: Clone, Cel: Copy, R> Drawing<CW, Cel, R> {
    pub fn compound_weight(&self, compound: GenericIndex<CW>) -> &CW {
        self.recording_geometry_with_rtree.compound_weight(compound)
    }

    pub fn compounds<'a, W: 'a>(
        &'a self,
        node: GenericIndex<W>,
    ) -> impl Iterator<Item = (Cel, GenericIndex<CW>)> + 'a {
        self.recording_geometry_with_rtree.compounds(node)
    }
}

#[debug_invariant(self.test_if_looses_dont_infringe_each_other())]
impl<CW: Clone, Cel: Copy, R: AccessRules> Drawing<CW, Cel, R> {
    pub fn new(
        rules: R,
        layer_count: usize,
        boundary: Polygon,
        place_boundary: Option<Polygon>,
    ) -> Self {
        let place_boundary = place_boundary.unwrap_or_else(|| boundary.clone());
        Self {
            recording_geometry_with_rtree: RecordingGeometryWithRtree::new(layer_count),
            rules,
            boundary,
            place_boundary,
        }
    }

    pub fn remove_band(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        band: BandTermsegIndex,
    ) -> Result<(), DrawingException> {
        let mut maybe_loose: Option<LooseIndex> = Some(match band {
            BandTermsegIndex::Lone(seg) => {
                self.recording_geometry_with_rtree
                    .remove_seg(recorder, seg.into());
                return Ok(());
            }
            BandTermsegIndex::Seq(first_loose_seg) => first_loose_seg.into(),
        });

        let mut dots = vec![];
        let mut segs = vec![];
        let mut bends = vec![];
        let mut outers = vec![];
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

                    let bend_primitive = self.primitive(bend);
                    let inner = bend_primitive.inner();

                    for outer in self.primitive(bend).outers().collect::<Vec<_>>() {
                        outers.push(outer);
                        self.reattach_bend(recorder, outer, inner);
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
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn add_fixed_dot(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        weight: FixedDotWeight,
    ) -> Result<FixedDotIndex, Infringement> {
        self.add_dot(recorder, weight, &|_drawing, _infringer, _infringee| true)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() - 1))]
    pub fn remove_fixed_dot(&mut self, recorder: &mut DrawingEdit<CW, Cel>, dot: FixedDotIndex) {
        self.recording_geometry_with_rtree
            .remove_dot(recorder, dot.into());
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        weight: FixedDotWeight,
    ) -> FixedDotIndex {
        self.add_dot_infringably(recorder, weight)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn add_dot<W: AccessDotWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        weight: W,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let dot = self.add_dot_infringably(recorder, weight);
        self.fail_and_remove_if_infringes_except(recorder, dot.into(), filter)?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn add_fixed_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, DrawingException> {
        self.add_seg(
            recorder,
            from.into(),
            to.into(),
            weight,
            &|_drawing, _infringer, _infringee| true,
        )
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    pub fn add_fixed_seg_infringably(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> FixedSegIndex {
        self.add_seg_infringably(recorder, from.into(), to.into(), weight)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn add_lone_loose_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, DrawingException> {
        let seg = self.add_seg(
            recorder,
            from.into(),
            to.into(),
            weight,
            &|_drawing, _infringer, _infringee| true,
        )?;
        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn add_seq_loose_seg(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, DrawingException> {
        let seg = self.add_seg(
            recorder,
            from,
            to.into(),
            weight,
            &|_drawing, _infringer, _infringee| true,
        )?;

        Ok(seg)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() - 1))]
    pub fn remove_termseg(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        termseg: BandTermsegIndex,
    ) {
        self.recording_geometry_with_rtree
            .remove_seg(recorder, termseg.into())
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn add_seg<W: AccessSegWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: DotIndex,
        to: DotIndex,
        weight: W,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<GenericIndex<W>, DrawingException>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let seg = self.add_seg_infringably(recorder, from, to, weight);
        self.fail_and_remove_if_infringes_except(recorder, seg.into(), filter)?;

        // Raise a collision exception if the currently created cane's seg
        // collides with:
        // - Different-net primitives,
        // - Same-net loose segs if the currently created cane is non-terminal.
        //
        // This solves two problems:
        //
        // It prevents the currently routed band from forming a
        // self-intersecting loop.
        //
        // And it prevents the currently routed band from infringing already
        // routed bands wrapped around the same core. This can happen because
        // when the band is squeezed through under bends the outward bows
        // of these bends are excluded from infringement detection to avoid
        // false positives (the code where this exception is made is in
        // `.update_this_and_outward_bows(...)`).
        //
        // XXX: Possible problem: What if there could be a collision due
        // to bend's length being zero? We may or may not want to create an
        // exception for this case, at least until we switch to our exact
        // lineocircular kernel Cyrk.
        if let Some(collision) =
            self.detect_collision_except(seg.into(), &|drawing, collider, collidee| {
                // Check whether the the seg is terminal, that is, whether at
                // least one of its two joints is a fixed dot.
                if matches!(from, DotIndex::Fixed(..)) || matches!(to, DotIndex::Fixed(..)) {
                    collider.primitive_ref(drawing).maybe_net()
                        != collidee.primitive_ref(drawing).maybe_net()
                } else {
                    // Cane is non-initial.
                    true
                }
            })
        {
            // Restore previous state.
            self.recording_geometry_with_rtree
                .remove_seg(recorder, seg.into().try_into().unwrap());
            return Err(collision.into());
        }

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn add_loose_bend(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: GearIndex,
        weight: LooseBendWeight,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<LooseBendIndex, DrawingException> {
        // It makes no sense to wrap something around or under one of its connectables.
        //
        if let Some(net) = weight.maybe_net() {
            if let Some(around_net) = around.primitive_ref(self).maybe_net() {
                if net == around_net {
                    return Err(AlreadyConnected(net, around.into()).into());
                }
            }
            //
            let mut outwards = around.ref_(self).outwards();
            while let Some(gear) = outwards.walk_next(self) {
                if let Some(next_gear_net) = gear.primitive_ref(self).maybe_net() {
                    if net == next_gear_net {
                        return Err(AlreadyConnected(net, gear.into()).into());
                    }
                }
            }
        }

        match around {
            GearIndex::FixedDot(core) => self
                .add_core_bend(recorder, from.into(), to.into(), core, weight, filter)
                .map_err(Into::into),
            GearIndex::FixedBend(around) => self
                .add_outer_bend(recorder, from, to, around.into(), weight, filter)
                .map_err(Into::into),
            GearIndex::LooseBend(around) => self
                .add_outer_bend(recorder, from, to, around.into(), weight, filter)
                .map_err(Into::into),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn add_core_bend<W: AccessBendWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: DotIndex,
        to: DotIndex,
        core: FixedDotIndex,
        weight: W,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        let bend =
            self.recording_geometry_with_rtree
                .add_bend(recorder, from, to, core.into(), weight);

        self.fail_and_remove_if_infringes_except(recorder, bend.into(), filter)?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn add_outer_bend(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: LooseDotIndex,
        to: LooseDotIndex,
        inner: BendIndex,
        weight: LooseBendWeight,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<GenericIndex<LooseBendWeight>, Infringement> {
        let core = match inner {
            BendIndex::Fixed(bend) => self.primitive(bend).core(),
            BendIndex::Loose(bend) => self.primitive(bend).core(),
        };
        let bend = self.recording_geometry_with_rtree.add_bend(
            recorder,
            from.into(),
            to.into(),
            core.into(),
            weight,
        );
        self.recording_geometry_with_rtree
            .reattach_bend(recorder, bend.into(), Some(inner));

        self.fail_and_remove_if_infringes_except(recorder, bend.into(), filter)?;
        Ok(bend)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn flip_bend(&mut self, recorder: &mut DrawingEdit<CW, Cel>, bend: FixedBendIndex) {
        self.recording_geometry_with_rtree
            .flip_bend(recorder, bend.into());
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn reattach_bend(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        bend: LooseBendIndex,
        maybe_new_inner: Option<LooseBendIndex>,
    ) {
        self.recording_geometry_with_rtree.reattach_bend(
            recorder,
            bend.into(),
            maybe_new_inner.map(Into::into),
        );
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn insert_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        sense: RotationSense,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<Cane, DrawingException> {
        let outer_gears = around.ref_(self).outer_gears();
        let cane = self.add_cane(
            recorder,
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            sense,
            filter,
        )?;

        for gear in outer_gears {
            self.reattach_bend(recorder, gear, Some(cane.bend));
        }

        for outer in self.primitive(cane.bend).outers().collect::<Vec<_>>() {
            self.update_this_and_outward_bows(recorder, outer)
                .inspect_err(|_| {
                    let joint = self.primitive(cane.bend).other_joint(cane.dot);
                    self.remove_cane(recorder, &cane, joint);
                })?;
        }

        Ok(cane)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn update_this_and_outward_bows(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        around: LooseBendIndex,
    ) -> Result<(), DrawingException> {
        self.update_bow(recorder, around)?;

        let mut outwards = self.primitive(around).outwards();
        while let Some(rail) = outwards.walk_next(self) {
            self.update_bow(recorder, rail)?;
        }

        Ok(())
    }

    fn update_bow(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        rail: LooseBendIndex,
    ) -> Result<(), DrawingException> {
        let rail_primitive = self.primitive(rail);
        let joints = rail_primitive.joints();
        let width = rail_primitive.width();

        let from_head = self.rear_head(joints.1);
        let to_head = self.rear_head(joints.0);

        let (from, to, offset) = if let Some(inner) = rail_primitive.inner() {
            let inner = inner.into();
            let from = self.guide_for_head_around_bend(
                &from_head,
                inner,
                RotationSense::Counterclockwise,
                width,
            )?;
            let to =
                self.guide_for_head_around_bend(&to_head, inner, RotationSense::Clockwise, width)?;
            let offset = self.offset_for_guide_for_head_around_bend(&from_head, inner, width);
            (from, to, offset)
        } else {
            let core = rail_primitive.core().into();
            let from = self.guide_for_head_around_dot(
                &from_head,
                core,
                RotationSense::Counterclockwise,
                width,
            )?;
            let to =
                self.guide_for_head_around_dot(&to_head, core, RotationSense::Clockwise, width)?;
            let offset = self.offset_for_guide_for_head_around_dot(&from_head, core, width);
            (from, to, offset)
        };

        let rail_outer_bows = self.collect_bend_outward_bows(rail);

        // Commenting out these two makes the crash go away.
        self.move_dot_with_infringement_filtering(
            recorder,
            joints.0.into(),
            from.end_point(),
            &|_drawing, _infringer, infringee| rail_outer_bows.contains(&infringee),
        )?;
        self.move_dot_with_infringement_filtering(
            recorder,
            joints.1.into(),
            to.end_point(),
            &|_drawing, _infringer, infringee| rail_outer_bows.contains(&infringee),
        )?;

        self.shift_bend_with_infringement_filtering(
            recorder,
            rail.into(),
            offset,
            &|_drawing, _infringer, infringee| rail_outer_bows.contains(&infringee),
        )?;

        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn add_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        from: DotIndex,
        around: GearIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        sense: RotationSense,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<Cane, DrawingException> {
        let seg_to = self.add_dot(recorder, dot_weight, filter)?;
        // we just checked that we can insert a dot there
        let to = self.add_dot_infringably(recorder, dot_weight);

        let seg = self
            .add_seg(
                recorder,
                from,
                seg_to.into(),
                seg_weight,
                &|drawing, infringer, infringee| {
                    filter(drawing, infringer, infringee)
                        // Don't infringe upon limbs of the current wraparound.
                        && !PrimitiveIndex::from(around)
                            .primitive_ref(drawing)
                            .limbs()
                            .contains(&infringee)
                },
            )
            .inspect_err(|_| {
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, to.into());
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, seg_to.into());
            })?;

        let (bend_from, bend_to) = match sense {
            RotationSense::Counterclockwise => (seg_to, to),
            RotationSense::Clockwise => (to, seg_to),
        };

        let bend = self
            .add_loose_bend(recorder, bend_from, bend_to, around, bend_weight, filter)
            .inspect_err(|_| {
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, to.into());
                self.recording_geometry_with_rtree
                    .remove_seg(recorder, seg.into());
                self.recording_geometry_with_rtree
                    .remove_dot(recorder, seg_to.into());
            })?;

        #[cfg(debug_assertions)]
        use crate::geometry::shape::MeasureLength;
        #[cfg(debug_assertions)]
        approx::assert_abs_diff_eq!(bend.primitive_ref(self).shape().length(), 0.0);

        Ok(Cane {
            seg,
            dot: seg_to,
            bend,
        })
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() - 4))]
    pub fn remove_cane(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        cane: &Cane,
        face: LooseDotIndex,
    ) {
        let outers = self.primitive(cane.bend).outers().collect::<Vec<_>>();

        // Removing a loose bend affects its outer bends.
        for outer in &outers {
            self.reattach_bend(recorder, *outer, self.primitive(cane.bend).inner());
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

        for outer in outers {
            self.update_this_and_outward_bows(recorder, outer).unwrap(); // Must never fail.
        }
    }

    pub fn cane(&self, dot: LooseDotIndex) -> Cane {
        Cane::from_dot(dot, self)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    pub fn move_dot(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        dot: DotIndex,
        to: Point,
    ) -> Result<(), Infringement> {
        match dot {
            DotIndex::Fixed(..) => self.move_dot_with_infringement_filtering(
                recorder,
                dot,
                to,
                &|_drawing, _infringer, _infringee| true,
            ),
            DotIndex::Loose(..) => self.move_dot_with_infringement_filtering(
                recorder,
                dot,
                to,
                &|_drawing, _infringer, _infringee| true,
            ),
        }
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn move_dot_with_infringement_filtering(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        dot: DotIndex,
        to: Point,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<(), Infringement> {
        let old_pos = self
            .recording_geometry_with_rtree
            .geometry()
            .dot_weight(dot)
            .pos();
        self.recording_geometry_with_rtree
            .move_dot(recorder, dot, to);

        for limb in dot.primitive_ref(self).limbs() {
            if let Some(infringement) = self.find_infringement_except(limb, filter) {
                // Restore previous state.
                self.recording_geometry_with_rtree
                    .move_dot(recorder, dot, old_pos);
                return Err(infringement);
            }
        }

        if let Some(infringement) = self.find_infringement_except(dot.into(), filter) {
            // Restore previous state.
            self.recording_geometry_with_rtree
                .move_dot(recorder, dot, old_pos);
            return Err(infringement);
        }

        Ok(())
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    fn shift_bend_with_infringement_filtering(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        bend: BendIndex,
        offset: f64,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<(), Infringement> {
        let old_offset = self
            .recording_geometry_with_rtree
            .geometry()
            .bend_weight(bend)
            .offset();
        self.recording_geometry_with_rtree
            .shift_bend(recorder, bend, offset);

        if let Some(infringement) = self.find_infringement_except(bend.into(), filter) {
            // Restore previous state.
            self.recording_geometry_with_rtree
                .shift_bend(recorder, bend, old_offset);
            return Err(infringement);
        }

        Ok(())
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules> Drawing<CW, Cel, R> {
    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    fn add_dot_infringably<W: AccessDotWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        weight: W,
    ) -> GenericIndex<W>
    where
        GenericIndex<W>: Into<PrimitiveIndex> + Copy,
    {
        self.recording_geometry_with_rtree.add_dot(recorder, weight)
    }

    #[debug_ensures(self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() + 1))]
    fn add_seg_infringably<W: AccessSegWeight + Into<PrimitiveWeight> + GetLayer>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
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

    pub fn add_compound(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        weight: CW,
    ) -> GenericIndex<CW> {
        self.recording_geometry_with_rtree
            .add_compound(recorder, weight)
    }

    pub fn remove_compound(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        compound: GenericIndex<CW>,
    ) {
        self.recording_geometry_with_rtree
            .remove_compound(recorder, compound);
    }

    pub fn add_to_compound<W>(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        primitive: GenericIndex<W>,
        entry_label: Cel,
        compound: GenericIndex<CW>,
    ) {
        self.recording_geometry_with_rtree.add_to_compound(
            recorder,
            primitive,
            entry_label,
            compound,
        );
    }

    #[debug_ensures(ret.is_ok() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count()))]
    #[debug_ensures(ret.is_err() -> self.recording_geometry_with_rtree.node_count() == old(self.recording_geometry_with_rtree.node_count() - 1))]
    fn fail_and_remove_if_infringes_except(
        &mut self,
        recorder: &mut DrawingEdit<CW, Cel>,
        node: PrimitiveIndex,
        filter: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.find_infringement_except(node, filter) {
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

    pub fn is_node_in_layer(
        &self,
        index: GenericNode<PrimitiveIndex, GenericIndex<CW>>,
        active_layer: usize,
    ) -> bool
    where
        CW: super::graph::IsInLayer,
    {
        match index {
            GenericNode::Primitive(primitive) => {
                primitive.primitive_ref(self).layer() == active_layer
            }
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
                primitive.primitive_ref(self).is_in_any_layer_of(layers)
            }
            GenericNode::Compound(compound) => {
                self.compound_weight(compound).is_in_any_layer_of(layers)
            }
        }
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
                self.infringements_among(
                    node,
                    self.locate_possible_infringees(node)
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
                .next()
                .is_some()
            })
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules>
    ApplyGeometryEdit<
        DotWeight,
        SegWeight,
        BendWeight,
        CW,
        Cel,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Drawing<CW, Cel, R>
{
    fn apply(&mut self, edit: &DrawingEdit<CW, Cel>) {
        self.recording_geometry_with_rtree.apply(edit);
    }
}
