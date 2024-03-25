use contracts::debug_ensures;
use enum_dispatch::enum_dispatch;
use geo::Point;

use rstar::{RTreeObject, AABB};
use thiserror::Error;

use super::graph::GetLayer;
use super::loose::{GetNextLoose, Loose, LooseIndex};
use super::rules::RulesTrait;
use super::segbend::Segbend;
use crate::drawing::bend::BendIndex;
use crate::drawing::collect::Collect;
use crate::drawing::dot::DotWeight;
use crate::drawing::graph::GetMaybeNet;
use crate::drawing::guide::Guide;
use crate::drawing::primitive::GetLimbs;
use crate::drawing::rules::GetConditions;
use crate::drawing::{
    bend::{FixedBendIndex, LooseBendIndex, LooseBendWeight},
    dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    graph::{GeometryIndex, GeometryWeight, MakePrimitive},
    grouping::{GroupingIndex, GroupingWeight},
    primitive::{GenericPrimitive, GetCore, GetInnerOuter, GetJoints, GetOtherJoint, MakeShape},
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
        SeqLooseSegIndex, SeqLooseSegWeight,
    },
};
use crate::geometry::{
    shape::{Shape, ShapeTrait},
    with_rtree::GeometryWithRtree,
    BendWeightTrait, DotWeightTrait, Geometry, GeometryLabel, GetOffset, GetPos, GetWidth,
    SegWeightTrait,
};
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::math::NoTangents;
use crate::wraparoundable::{GetWraparound, Wraparoundable, WraparoundableIndex};

use super::bend::BendWeight;
use super::seg::SegWeight;

#[enum_dispatch]
#[derive(Error, Debug, Clone, Copy)]
pub enum LayoutException {
    #[error(transparent)]
    NoTangents(#[from] NoTangents),
    #[error(transparent)]
    Infringement(#[from] Infringement),
    #[error(transparent)]
    Collision(#[from] Collision),
    #[error(transparent)]
    AlreadyConnected(#[from] AlreadyConnected),
}

// TODO add real error messages + these should eventually use Display
#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} infringes on {1:?}")]
pub struct Infringement(pub Shape, pub GeometryIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{0:?} collides with {1:?}")]
pub struct Collision(pub Shape, pub GeometryIndex);

#[derive(Error, Debug, Clone, Copy)]
#[error("{1:?} is already connected to net {0}")]
pub struct AlreadyConnected(pub usize, pub GeometryIndex);

#[derive(Debug)]
pub struct Drawing<R: RulesTrait> {
    geometry_with_rtree: GeometryWithRtree<
        GeometryWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        GroupingWeight,
        GeometryIndex,
        DotIndex,
        SegIndex,
        BendIndex,
        GroupingIndex,
    >,
    rules: R,
}

impl<R: RulesTrait> Drawing<R> {
    pub fn new(rules: R) -> Self {
        Self {
            geometry_with_rtree: GeometryWithRtree::new(2),
            rules,
        }
    }

    pub fn remove_band(&mut self, first_loose: SeqLooseSegIndex) {
        let mut dots = vec![];
        let mut segs = vec![];
        let mut bends = vec![];
        let mut outers = vec![];

        let mut maybe_loose = Some(first_loose.into());
        let mut prev = None;

        while let Some(loose) = maybe_loose {
            match loose {
                LooseIndex::Dot(dot) => {
                    dots.push(dot);
                }
                LooseIndex::LoneSeg(seg) => {
                    self.geometry_with_rtree.remove_seg(seg.into());
                    break;
                }
                LooseIndex::SeqSeg(seg) => {
                    segs.push(seg);
                }
                LooseIndex::Bend(bend) => {
                    bends.push(bend);

                    if let Some(outer) = self.primitive(bend).outer() {
                        outers.push(outer);
                        self.reattach_bend(outer, self.primitive(bend).inner());
                    }
                }
            }

            let prev_prev = prev;
            prev = maybe_loose;
            maybe_loose = self.loose(loose).next_loose(prev_prev);
        }

        for bend in bends {
            self.geometry_with_rtree.remove_bend(bend.into());
        }

        for seg in segs {
            self.geometry_with_rtree.remove_seg(seg.into());
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we first need unchanged to remove the segs and bends from the R-tree.

        for dot in dots {
            self.geometry_with_rtree.remove_dot(dot.into());
        }

        for outer in outers {
            self.update_this_and_outward_bows(outer).unwrap(); // Must never fail.
        }
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() - 4))]
    pub fn remove_segbend(&mut self, segbend: &Segbend, face: LooseDotIndex) {
        let maybe_outer = self.primitive(segbend.bend).outer();

        // Removing a loose bend affects its outer bends.
        if let Some(outer) = maybe_outer {
            self.reattach_bend(outer, self.primitive(segbend.bend).inner());
        }

        self.geometry_with_rtree.remove_bend(segbend.bend.into());
        self.geometry_with_rtree.remove_seg(segbend.seg.into());

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we first need unchanged to remove the segs and bends from the R-tree.

        self.geometry_with_rtree.remove_dot(face.into());
        self.geometry_with_rtree.remove_dot(segbend.dot.into());

        if let Some(outer) = maybe_outer {
            self.update_this_and_outward_bows(outer).unwrap(); // Must never fail.
        }
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.add_dot_infringably(weight, None)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    fn add_dot_infringably<W: DotWeightTrait<GeometryWeight> + GetLayer>(
        &mut self,
        weight: W,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let dot = self.geometry_with_rtree.add_dot(weight);
        self.fail_and_remove_if_infringes_except(dot.into(), infringables)?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, Infringement> {
        self.add_seg_infringably(from.into(), to.into(), weight, None)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() >= old(self.geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn insert_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Segbend, LayoutException> {
        let maybe_wraparound = self.wraparoundable(around).wraparound();
        let infringables = self.collect().wraparounded_bows(around);

        let segbend = self.add_segbend_infringably(
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
            Some(&infringables),
        )?;

        if let Some(wraparound) = maybe_wraparound {
            self.reattach_bend(wraparound, Some(segbend.bend));
        }

        if let Some(outer) = self.primitive(segbend.bend).outer() {
            self.update_this_and_outward_bows(outer).map_err(|err| {
                let joint = self.primitive(segbend.bend).other_joint(segbend.dot);
                self.remove_segbend(&segbend, joint.into());
                err
            })?;
        }

        // Segs must not cross.
        if let Some(collision) = self.detect_collision(segbend.seg.into()) {
            let joint = self.primitive(segbend.bend).other_joint(segbend.dot);
            self.remove_segbend(&segbend, joint.into());
            return Err(collision.into());
        }

        Ok::<Segbend, LayoutException>(segbend)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count())
        || self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() - 1)
        || self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 1))]
    fn reattach_bend(&mut self, bend: LooseBendIndex, maybe_new_inner: Option<LooseBendIndex>) {
        self.geometry_with_rtree
            .reattach_bend(bend.into(), maybe_new_inner.map(Into::into));
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn update_this_and_outward_bows(
        &mut self,
        around: LooseBendIndex,
    ) -> Result<(), LayoutException> {
        // FIXME: Fail gracefully on infringement.
        let mut maybe_rail = Some(around);

        while let Some(rail) = maybe_rail {
            let rail_primitive = self.primitive(rail);
            let joints = rail_primitive.joints();

            let guide = Guide::new(self);
            let from_head = guide.rear_head(joints.1);
            let to_head = guide.rear_head(joints.0);

            if let Some(inner) = rail_primitive.inner() {
                let from = guide
                    .head_around_bend_segment(
                        &from_head.into(),
                        inner.into(),
                        true,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let to = guide
                    .head_around_bend_segment(
                        &to_head.into(),
                        inner.into(),
                        false,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let offset = guide.head_around_bend_offset(
                    &from_head.into(),
                    inner.into(),
                    self.primitive(rail).width(),
                );

                self.move_dot_infringably(
                    joints.0.into(),
                    from,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;
                self.move_dot_infringably(
                    joints.1.into(),
                    to,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;

                self.shift_bend_infringably(
                    rail.into(),
                    offset,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;

                // Update offsets in case the rule conditions changed.
            } else {
                let core = rail_primitive.core();
                let from = guide
                    .head_around_dot_segment(
                        &from_head.into(),
                        core.into(),
                        true,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let to = guide
                    .head_around_dot_segment(
                        &to_head.into(),
                        core.into(),
                        false,
                        self.primitive(rail).width(),
                    )?
                    .end_point();
                let offset = guide.head_around_dot_offset(
                    &from_head.into(),
                    core.into(),
                    self.primitive(rail).width(),
                );

                self.move_dot_infringably(
                    joints.0.into(),
                    from,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;
                self.move_dot_infringably(
                    joints.1.into(),
                    to,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;

                self.shift_bend_infringably(
                    rail.into(),
                    offset,
                    Some(&self.collect().bend_outer_bows(rail)),
                )?;
            }

            maybe_rail = self.primitive(rail).outer();
        }

        Ok::<(), LayoutException>(())
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() >= old(self.geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_segbend(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
    ) -> Result<Segbend, LayoutException> {
        self.add_segbend_infringably(
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
            Some(&self.collect().wraparounded_bows(around)),
        )
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() >= old(self.geometry_with_rtree.graph().edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_segbend_infringably(
        &mut self,
        from: DotIndex,
        around: WraparoundableIndex,
        dot_weight: LooseDotWeight,
        seg_weight: SeqLooseSegWeight,
        bend_weight: LooseBendWeight,
        cw: bool,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<Segbend, LayoutException> {
        let seg_to = self.add_dot_infringably(dot_weight, infringables)?;
        let seg = self
            .add_seg_infringably(from, seg_to.into(), seg_weight, infringables)
            .map_err(|err| {
                self.geometry_with_rtree.remove_dot(seg_to.into());
                err
            })?;

        let to = self
            .add_dot_infringably(dot_weight, infringables)
            .map_err(|err| {
                self.geometry_with_rtree.remove_seg(seg.into());
                self.geometry_with_rtree.remove_dot(seg_to.into());
                err
            })?;

        let (bend_from, bend_to) = if cw { (to, seg_to) } else { (seg_to, to) };

        let bend = self
            .add_loose_bend_infringably(bend_from, bend_to, around, bend_weight, infringables)
            .map_err(|err| {
                self.geometry_with_rtree.remove_dot(to.into());
                self.geometry_with_rtree.remove_seg(seg.into());
                self.geometry_with_rtree.remove_dot(seg_to.into());
                err
            })?;

        Ok::<Segbend, LayoutException>(Segbend {
            seg,
            dot: seg_to,
            bend,
        })
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_lone_loose_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: LoneLooseSegWeight,
    ) -> Result<LoneLooseSegIndex, Infringement> {
        let seg = self.add_seg_infringably(from.into(), to.into(), weight, Some(&[]))?;
        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_seq_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: SeqLooseSegWeight,
    ) -> Result<SeqLooseSegIndex, Infringement> {
        let seg = self.add_seg_infringably(from, to.into(), weight, Some(&[]))?;
        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_seg_infringably<W: SegWeightTrait<GeometryWeight> + GetLayer>(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        weight: W,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let seg = self.geometry_with_rtree.add_seg(from, to, weight);
        self.fail_and_remove_if_infringes_except(seg.into(), infringables)?;

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 3)
        || self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_loose_bend_infringably(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: WraparoundableIndex,
        weight: LooseBendWeight,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<LooseBendIndex, LayoutException> {
        // It makes no sense to wrap something around or under one of its connectables.
        //
        if let Some(net) = weight.maybe_net {
            if let Some(around_net) = around.primitive(self).maybe_net() {
                if net == around_net {
                    return Err(AlreadyConnected(net, around.into()).into());
                }
            }
            //
            if let Some(wraparound) = self.wraparoundable(around).wraparound() {
                if let Some(wraparound_net) = wraparound.primitive(self).maybe_net() {
                    if net == wraparound_net {
                        return Err(AlreadyConnected(net, wraparound.into()).into());
                    }
                }
            }
        }

        match around {
            WraparoundableIndex::FixedDot(core) => self
                .add_core_bend_infringably(from.into(), to.into(), core, weight, infringables)
                .map_err(Into::into),
            WraparoundableIndex::FixedBend(around) => self
                .add_outer_bend_infringably(from, to, around.into(), weight, infringables)
                .map_err(Into::into),
            WraparoundableIndex::LooseBend(around) => self
                .add_outer_bend_infringably(from, to, around.into(), weight, infringables)
                .map_err(Into::into),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_core_bend_infringably<W: BendWeightTrait<GeometryWeight> + GetLayer>(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        core: FixedDotIndex,
        weight: W,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let bend = self
            .geometry_with_rtree
            .add_bend(from, to, core.into(), weight);

        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_outer_bend_infringably(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        inner: BendIndex,
        weight: LooseBendWeight,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<GenericIndex<LooseBendWeight>, Infringement> {
        let core = *self
            .geometry_with_rtree
            .graph()
            .neighbors(inner.node_index())
            .filter(|ni| {
                matches!(
                    self.geometry_with_rtree
                        .graph()
                        .edge_weight(
                            self.geometry_with_rtree
                                .graph()
                                .find_edge(inner.node_index(), *ni)
                                .unwrap()
                        )
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|ni| FixedDotIndex::new(ni))
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = self
            .geometry_with_rtree
            .add_bend(from.into(), to.into(), core.into(), weight);
        self.geometry_with_rtree
            .reattach_bend(bend.into(), Some(inner));

        self.fail_and_remove_if_infringes_except(bend.into(), infringables)?;
        Ok(bend)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn flip_bend(&mut self, bend: FixedBendIndex) {
        self.geometry_with_rtree.flip_bend(bend.into());
    }

    pub fn segbend(&self, dot: LooseDotIndex) -> Segbend {
        Segbend::from_dot(dot, self)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() - 1))]
    fn fail_and_remove_if_infringes_except(
        &mut self,
        node: GeometryIndex,
        maybe_except: Option<&[GeometryIndex]>,
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.detect_infringement_except(node, maybe_except) {
            if let Ok(dot) = node.try_into() {
                self.geometry_with_rtree.remove_dot(dot);
            } else if let Ok(seg) = node.try_into() {
                self.geometry_with_rtree.remove_seg(seg);
            } else if let Ok(bend) = node.try_into() {
                self.geometry_with_rtree.remove_bend(bend);
            }
            return Err(infringement);
        }
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = GeometryIndex> + '_ {
        self.geometry_with_rtree
            .rtree()
            .iter()
            .map(|wrapper| wrapper.data)
    }

    pub fn layer_nodes(&self, layer: u64) -> impl Iterator<Item = GeometryIndex> + '_ {
        self.geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&AABB::from_corners(
                [-f64::INFINITY, -f64::INFINITY, layer as f64],
                [f64::INFINITY, f64::INFINITY, layer as f64],
            ))
            .map(|wrapper| wrapper.data)
    }

    pub fn node_count(&self) -> usize {
        self.geometry_with_rtree.graph().node_count()
    }
}

impl<R: RulesTrait> Drawing<R> {
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), Infringement> {
        match dot {
            DotIndex::Fixed(..) => self.move_dot_infringably(dot, to, Some(&[])),
            DotIndex::Loose(..) => self.move_dot_infringably(dot, to, Some(&[])),
        }
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn move_dot_infringably(
        &mut self,
        dot: DotIndex,
        to: Point,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<(), Infringement> {
        let old_pos = self.geometry_with_rtree.geometry().dot_weight(dot).pos();
        self.geometry_with_rtree.move_dot(dot, to);

        for limb in dot.primitive(self).limbs() {
            if let Some(infringement) = self.detect_infringement_except(limb.into(), infringables) {
                // Restore original state.
                self.geometry_with_rtree.move_dot(dot, old_pos);
                return Err(infringement);
            }
        }

        if let Some(infringement) = self.detect_infringement_except(dot.into(), infringables) {
            // Restore original state.
            self.geometry_with_rtree.move_dot(dot, old_pos);
            return Err(infringement);
        }

        Ok(())
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn shift_bend_infringably(
        &mut self,
        bend: BendIndex,
        offset: f64,
        infringables: Option<&[GeometryIndex]>,
    ) -> Result<(), Infringement> {
        let old_offset = self
            .geometry_with_rtree
            .geometry()
            .bend_weight(bend)
            .offset();
        self.geometry_with_rtree.shift_bend(bend, offset);

        if let Some(infringement) = self.detect_infringement_except(bend.into(), infringables) {
            // Restore original state.
            self.geometry_with_rtree.shift_bend(bend, old_offset);
            return Err(infringement);
        }

        Ok(())
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn detect_infringement_except(
        &self,
        node: GeometryIndex,
        maybe_except: Option<&[GeometryIndex]>,
    ) -> Option<Infringement> {
        let limiting_shape = node.primitive(self).shape().inflate(
            node.primitive(self)
                .maybe_net()
                .and_then(|net| Some(self.rules.largest_clearance(Some(net))))
                .unwrap_or(0.0),
        );
        let mut inflated_shape = limiting_shape; // Unused temporary value just for initialization.
        let conditions = node.primitive(self).conditions();

        self.geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&limiting_shape.full_height_envelope_3d(0.0, 2))
            .filter(|wrapper| maybe_except.is_some_and(|except| !except.contains(&wrapper.data)))
            .filter(|wrapper| !self.are_connectable(node, wrapper.data))
            .filter(|wrapper| {
                let infringee_conditions = wrapper.data.primitive(self).conditions();

                let epsilon = 1.0;
                inflated_shape = node.primitive(self).shape().inflate(
                    (self.rules.clearance(&conditions, &infringee_conditions) - epsilon)
                        .clamp(0.0, f64::INFINITY),
                );

                inflated_shape.intersects(&wrapper.data.primitive(self).shape())
            })
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|infringee| Some(Infringement(inflated_shape, infringee)))
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn detect_collision(&self, node: GeometryIndex) -> Option<Collision> {
        let shape = node.primitive(self).shape();

        self.geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&shape.full_height_envelope_3d(0.0, 2))
            .filter(|wrapper| !self.are_connectable(node, wrapper.data))
            .filter(|wrapper| shape.intersects(&wrapper.data.primitive(self).shape()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|collidee| Some(Collision(shape, collidee)))
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn are_connectable(&self, node1: GeometryIndex, node2: GeometryIndex) -> bool {
        if let (Some(node1_net_id), Some(node2_net_id)) = (
            node1.primitive(self).maybe_net(),
            node2.primitive(self).maybe_net(),
        ) {
            node1_net_id == node2_net_id
        } else {
            true
        }
    }
}

impl<R: RulesTrait> Drawing<R> {
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn geometry(
        &self,
    ) -> &Geometry<
        GeometryWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        GroupingWeight,
        GeometryIndex,
        DotIndex,
        SegIndex,
        BendIndex,
        GroupingIndex,
    > {
        self.geometry_with_rtree.geometry()
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn rules(&self) -> &R {
        &self.rules
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn guide(&self) -> Guide<R> {
        Guide::new(self)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn collect(&self) -> Collect<R> {
        Collect::new(self)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W, R> {
        GenericPrimitive::new(index, self)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn wraparoundable(&self, index: WraparoundableIndex) -> Wraparoundable<R> {
        Wraparoundable::new(index, self)
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn loose(&self, index: LooseIndex) -> Loose<R> {
        Loose::new(index, self)
    }
}
