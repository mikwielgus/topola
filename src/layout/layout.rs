use contracts::debug_ensures;
use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;

use rstar::RTreeObject;
use thiserror::Error;

use super::band::Band;
use super::connectivity::{
    BandIndex, BandWeight, ComponentIndex, ComponentWeight, ConnectivityGraph, ConnectivityLabel,
    ConnectivityWeight, GetNet,
};
use super::geometry::with_rtree::GeometryWithRtree;
use super::loose::{GetNextLoose, Loose, LooseIndex};
use super::rules::RulesTrait;
use super::segbend::Segbend;
use crate::graph::{GenericIndex, GetNodeIndex};
use crate::layout::bend::BendIndex;
use crate::layout::collect::Collect;
use crate::layout::dot::DotWeight;
use crate::layout::geometry::{
    BendWeightTrait, DotWeightTrait, Geometry, GeometryLabel, GetOffset, GetPos, GetWidth,
    SegWeightTrait,
};
use crate::layout::guide::Guide;
use crate::layout::primitive::GetLimbs;
use crate::layout::rules::{Conditions, GetConditions};
use crate::layout::{
    bend::{FixedBendIndex, LooseBendIndex, LooseBendWeight},
    dot::{DotIndex, FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    geometry::shape::{Shape, ShapeTrait},
    graph::{GeometryIndex, GeometryWeight, GetComponentIndex, MakePrimitive},
    primitive::{
        GenericPrimitive, GetCore, GetInnerOuter, GetJoints, GetOtherJoint, GetWeight, MakeShape,
    },
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SegIndex,
        SeqLooseSegIndex, SeqLooseSegWeight,
    },
};
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
pub struct AlreadyConnected(pub i64, pub GeometryIndex);

#[derive(Debug)]
pub struct Layout<R: RulesTrait> {
    geometry_with_rtree: GeometryWithRtree<
        GeometryWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        GeometryIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    >,
    connectivity: ConnectivityGraph,
    rules: R,
}

impl<R: RulesTrait> Layout<R> {
    pub fn new(rules: R) -> Self {
        Self {
            geometry_with_rtree: GeometryWithRtree::new(),
            connectivity: StableDiGraph::default(),
            rules,
        }
    }

    pub fn remove_band(&mut self, band: BandIndex) {
        let mut dots = vec![];
        let mut segs = vec![];
        let mut bends = vec![];
        let mut outers = vec![];

        let from = self.band(band).from();
        let mut maybe_loose = self.primitive(from).first_loose(band);
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

        self.connectivity.remove_node(band.node_index());
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

    // TODO: This method shouldn't be public.
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_component(&mut self, net: i64) -> ComponentIndex {
        ComponentIndex::new(
            self.connectivity
                .add_node(ConnectivityWeight::Component(ComponentWeight { net })),
        )
    }

    // TODO: This method shouldn't be public.
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_band(&mut self, from: FixedDotIndex, width: f64) -> BandIndex {
        BandIndex::new(
            self.connectivity
                .add_node(ConnectivityWeight::Band(BandWeight {
                    width,
                    net: self.primitive(from).net(),
                    from,
                })),
        )
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, Infringement> {
        self.add_dot_infringably(weight, &[])
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_dot(&mut self, weight: LooseDotWeight) -> Result<LooseDotIndex, ()> {
        self.add_dot_infringably(weight, &[])
    }*/

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    fn add_dot_infringably<W: DotWeightTrait<GeometryWeight>>(
        &mut self,
        weight: W,
        infringables: &[GeometryIndex],
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
        self.add_seg_infringably(from.into(), to.into(), weight, &[])
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
        let mut infringables = self.collect().potential_segbend_abutters(from, around);

        if let Some(wraparound) = maybe_wraparound {
            infringables.append(&mut self.collect().outer_bows(wraparound));
        }

        let segbend = self.add_segbend_infringably(
            from,
            around,
            dot_weight,
            seg_weight,
            bend_weight,
            cw,
            &infringables,
        )?;

        if let Some(wraparound) = maybe_wraparound {
            self.reattach_bend(wraparound, Some(segbend.bend));
        }

        if let Some(outer) = self.primitive(segbend.bend).outer() {
            self.update_this_and_outward_bows(outer)?;
        }

        // Segs must not cross.
        if let Some(collision) = self.detect_collision(segbend.seg.into()) {
            let end = self.primitive(segbend.bend).other_joint(segbend.dot);
            self.remove_segbend(&segbend, end.into());
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
                    &self.collect().bend_abutters_and_posteriors(rail),
                )?;
                self.move_dot_infringably(
                    joints.1.into(),
                    to,
                    &self.collect().bend_abutters_and_posteriors(rail),
                )?;

                self.shift_bend_infringably(
                    rail.into(),
                    offset,
                    &self.collect().bend_abutters_and_posteriors(rail),
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
                    &self.collect().bend_abutters_and_posteriors(rail),
                )?;
                self.move_dot_infringably(
                    joints.1.into(),
                    to,
                    &self.collect().bend_abutters_and_posteriors(rail),
                )?;

                self.shift_bend_infringably(
                    rail.into(),
                    offset,
                    &self.collect().bend_abutters_and_posteriors(rail),
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
            &self.collect().potential_segbend_abutters(from, around),
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
        infringables: &[GeometryIndex],
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
        let seg = self.add_seg_infringably(from.into(), to.into(), weight, &[])?;

        self.connectivity.update_edge(
            self.primitive(from).component().node_index(),
            weight.band.node_index(),
            ConnectivityLabel::Band,
        );
        self.connectivity.update_edge(
            weight.band.node_index(),
            self.primitive(to).component().node_index(),
            ConnectivityLabel::Band,
        );

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
        let seg = self.add_seg_infringably(from, to.into(), weight, &[])?;

        if let DotIndex::Fixed(dot) = from {
            self.connectivity.update_edge(
                self.primitive(dot).component().node_index(),
                weight.band.node_index(),
                ConnectivityLabel::Band,
            );
        }

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(ret.is_err() -> self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn add_seg_infringably<W: SegWeightTrait<GeometryWeight>>(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        weight: W,
        infringables: &[GeometryIndex],
    ) -> Result<GenericIndex<W>, Infringement>
    where
        GenericIndex<W>: Into<GeometryIndex> + Copy,
    {
        let seg = self.geometry_with_rtree.add_seg(from, to, weight);
        self.fail_and_remove_if_infringes_except(seg.into(), infringables)?;

        Ok(seg)
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    pub fn add_fixed_bend(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        around: GeometryIndex,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        match around {
            GeometryIndex::FixedDot(core) => self.add_core_bend(from, to, core, weight),
            GeometryIndex::FixedBend(around) => self.add_outer_bend(from, to, around, weight),
            _ => unreachable!(),
        }
    }*/

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
        infringables: &[GeometryIndex],
    ) -> Result<LooseBendIndex, LayoutException> {
        // It makes no sense to wrap something around or under one of its connectables.
        let net = self.band(weight.band).net();
        //
        if net == around.primitive(self).net() {
            return Err(AlreadyConnected(net, around.into()).into());
        }
        //
        if let Some(wraparound) = self.wraparoundable(around).wraparound() {
            if net == wraparound.primitive(self).net() {
                return Err(AlreadyConnected(net, wraparound.into()).into());
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
    fn add_core_bend_infringably<W: BendWeightTrait<GeometryWeight>>(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        core: FixedDotIndex,
        weight: W,
        infringables: &[GeometryIndex],
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
        infringables: &[GeometryIndex],
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
        except: &[GeometryIndex],
    ) -> Result<(), Infringement> {
        if let Some(infringement) = self.detect_infringement_except(node, except) {
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

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes().map(|node| node.primitive(self).shape())
    }

    pub fn node_count(&self) -> usize {
        self.geometry_with_rtree.graph().node_count()
    }

    fn node_indices(&self) -> impl Iterator<Item = GeometryIndex> + '_ {
        self.geometry_with_rtree
            .rtree()
            .iter()
            .map(|wrapper| wrapper.data)
    }
}

impl<R: RulesTrait> Layout<R> {
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), Infringement> {
        match dot {
            DotIndex::Fixed(..) => self.move_dot_infringably(dot, to, &[]),
            DotIndex::Loose(loose) => self.move_dot_infringably(
                dot,
                to,
                &self.collect().bend_abutters(self.primitive(loose).bend()),
            ),
        }
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn move_dot_infringably(
        &mut self,
        dot: DotIndex,
        to: Point,
        infringables: &[GeometryIndex],
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
        infringables: &[GeometryIndex],
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
        except: &[GeometryIndex],
    ) -> Option<Infringement> {
        let limiting_shape = node
            .primitive(self)
            .shape()
            .inflate(self.rules.clearance_net_limit(node.primitive(self).net()));
        let mut inflated_shape = limiting_shape; // Unused temporary value just for initialization.
        let conditions = node.primitive(self).conditions();

        self.geometry_with_rtree
            .rtree()
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&limiting_shape))
            .filter(|wrapper| !self.are_connectable(node, wrapper.data))
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| {
                let infringee_conditions = wrapper.data.primitive(self).conditions();

                let epsilon = 1.0;
                inflated_shape = node.primitive(self).shape().inflate(
                    (self.rules.clearance(&conditions, &infringee_conditions) - epsilon)
                        .clamp(0.0, f64::INFINITY),
                );

                inflated_shape.intersects(wrapper.geom())
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
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| !self.are_connectable(node, wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
            .and_then(|collidee| Some(Collision(shape, collidee)))
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    fn are_connectable(&self, node1: GeometryIndex, node2: GeometryIndex) -> bool {
        let node1_net = node1.primitive(self).net();
        let node2_net = node2.primitive(self).net();

        (node1_net == node2_net) || node1_net == -1 || node2_net == -2
    }
}

impl<R: RulesTrait> Layout<R> {
    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn connectivity(&self) -> &ConnectivityGraph {
        &self.connectivity
    }

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn geometry(
        &self,
    ) -> &Geometry<
        GeometryWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        GeometryIndex,
        DotIndex,
        SegIndex,
        BendIndex,
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

    #[debug_ensures(self.geometry_with_rtree.graph().node_count() == old(self.geometry_with_rtree.graph().node_count()))]
    #[debug_ensures(self.geometry_with_rtree.graph().edge_count() == old(self.geometry_with_rtree.graph().edge_count()))]
    pub fn band(&self, index: BandIndex) -> Band<R> {
        Band::new(index, self)
    }
}
