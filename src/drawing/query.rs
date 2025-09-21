// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Line;
use petgraph::visit::Walker;
use specctra_core::rules::GetConditions;

use crate::{
    drawing::{
        graph::{GetMaybeNet, MakePrimitiveRef},
        primitive::MakePrimitiveShape,
        Collision, Infringement,
    },
    geometry::{
        primitive::{AccessPrimitiveShape, PrimitiveShape, SegShape},
        GenericNode, GetLayer,
    },
    graph::GenericIndex,
};

use super::{
    band::{BandTermsegIndex, BandUid},
    bend::LooseBendIndex,
    gear::WalkOutwards,
    graph::PrimitiveIndex,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::GetJoints,
    rules::AccessRules,
    Drawing,
};

#[derive(Clone, Debug, thiserror::Error)]
#[error("unable to resolve Loose to BandUid")]
pub struct BandUidError {
    pub maybe_end: Option<BandTermsegIndex>,
}

/// Routines implementing various queries on drawing. A query is a routine that
/// returns indices of one or more primitives.
impl<CW: Clone, Cel: Copy, R: AccessRules> Drawing<CW, Cel, R> {
    pub fn find_loose_band_uid(&self, start_loose: LooseIndex) -> Result<BandUid, BandUidError> {
        match (
            self.find_loose_band_first_seg(start_loose),
            self.find_loose_band_last_seg(start_loose),
        ) {
            (Some(first), Some(last)) => Ok(BandUid::from((first, last))),
            (Some(x), None) | (None, Some(x)) => Err(BandUidError { maybe_end: Some(x) }),
            (None, None) => Err(BandUidError { maybe_end: None }),
        }
    }

    fn find_loose_band_first_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex> {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return Some(BandTermsegIndex::Lone(seg));
        }

        let mut loose = start_loose;
        let mut prev = None;

        loop {
            if let Some(next_loose) = self.loose(loose).prev_loose(prev) {
                prev = Some(loose);
                loose = next_loose;
            } else {
                return loose.try_into().ok();
            }
        }
    }

    fn find_loose_band_last_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex> {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return Some(BandTermsegIndex::Lone(seg));
        }

        let mut loose = start_loose;
        let mut next = None;

        loop {
            if let Some(prev_loose) = self.loose(loose).next_loose(next) {
                next = Some(loose);
                loose = prev_loose;
            } else {
                return loose.try_into().ok();
            }
        }
    }

    pub fn collect_bend_outward_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];

        let mut outwards = self.primitive(bend).outwards();
        while let Some(next) = outwards.walk_next(self) {
            v.append(&mut self.collect_bend_bow(next));
        }

        v
    }

    fn collect_bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v: Vec<PrimitiveIndex> = vec![];
        v.push(bend.into());

        let joints = self.primitive(bend).joints();
        v.push(joints.0.into());
        v.push(joints.1.into());

        if let Some(seg0) = self.primitive(joints.0).seg() {
            v.push(seg0.into());
        }

        if let Some(seg1) = self.primitive(joints.1).seg() {
            v.push(seg1.into());
        }

        v
    }

    pub fn cut(
        &self,
        line: Line,
        width: f64,
        layer: usize,
    ) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        let limiting_shape = PrimitiveShape::Seg(SegShape {
            from: line.start_point(),
            to: line.end_point(),
            width,
        })
        .inflate(self.rules().largest_clearance(None));

        self.recording_geometry_with_rtree()
            .rtree()
            .locate_in_envelope_intersecting(&limiting_shape.envelope_3d(width, layer))
            .filter_map(|wrapper| {
                if let GenericNode::Primitive(primitive_node) = wrapper.data {
                    Some(primitive_node)
                } else {
                    None
                }
            })
            .filter_map(move |primitive_node| {
                limiting_shape
                    .intersects(&primitive_node.primitive_ref(self).shape())
                    .then_some(primitive_node)
            })
    }

    pub(super) fn find_infringement_except<'a>(
        &'a self,
        infringer: PrimitiveIndex,
        predicate: &'a impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Option<Infringement> {
        self.infringements_except(infringer, predicate).next()
    }

    /*pub(super) fn infringements<'a>(
        &'a self,
        infringer: PrimitiveIndex,
    ) -> impl Iterator<Item = Infringement> + 'a {
        self.infringements_except(infringer, &|_, _, _| true)
    }*/

    fn infringements_except<'a>(
        &'a self,
        infringer: PrimitiveIndex,
        predicate: &'a impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> impl Iterator<Item = Infringement> + 'a {
        self.infringements_among(
            infringer,
            self.locate_possible_infringees(infringer)
                .filter_map(move |infringee_node| {
                    if let GenericNode::Primitive(primitive_node) = infringee_node {
                        Some(primitive_node)
                    } else {
                        None
                    }
                })
                .filter(move |infringee| predicate(&self, infringer, *infringee)),
        )
    }

    pub(super) fn infringements_among<'a>(
        &'a self,
        infringer: PrimitiveIndex,
        it: impl Iterator<Item = PrimitiveIndex> + 'a,
    ) -> impl Iterator<Item = Infringement> + 'a {
        self.overlapees_among(infringer, it)
            .filter(move |infringement| {
                // Infringement with loose dots resulted in false positives for
                // line-of-sight paths.
                !matches!(infringer, PrimitiveIndex::LooseDot(..))
                    && !matches!(infringement.2, PrimitiveIndex::LooseDot(..))
            })
            .filter(move |infringement| !self.are_connectable(infringer, infringement.2))
    }

    pub fn overlapees<'a>(
        &'a self,
        overlapper: PrimitiveIndex,
    ) -> impl Iterator<Item = Infringement> + 'a {
        self.overlapees_among(
            overlapper,
            self.locate_possible_infringees(overlapper)
                .filter_map(move |infringee_node| {
                    if let GenericNode::Primitive(primitive_node) = infringee_node {
                        Some(primitive_node)
                    } else {
                        None
                    }
                })
                .filter(move |&overlapee| overlapper != overlapee),
        )
    }

    pub(super) fn overlapees_among<'a>(
        &'a self,
        intersector: PrimitiveIndex,
        it: impl Iterator<Item = PrimitiveIndex> + 'a,
    ) -> impl Iterator<Item = Infringement> + 'a {
        let conditions = intersector.primitive_ref(self).conditions();

        it.filter_map(move |primitive_node| {
            let infringee_conditions = primitive_node.primitive_ref(self).conditions();

            let epsilon = 1.0;
            let inflated_infringer_shape = intersector.primitive_ref(self).shape().inflate(match (
                &conditions,
                infringee_conditions,
            ) {
                (None, _) | (_, None) => return None,
                (Some(lhs), Some(rhs)) => {
                    let Some(clearance) = self.rules().clearance(lhs, &rhs) else {
                        return None;
                    };

                    // Note the epsilon comparison.
                    // XXX: Epsilon is probably too large. But what should
                    // it be exactly then?
                    (clearance - epsilon).clamp(0.0, f64::INFINITY)
                }
            });

            let infringee_shape = primitive_node.primitive_ref(self).shape();

            inflated_infringer_shape
                .intersects(&infringee_shape)
                .then_some(Infringement(
                    inflated_infringer_shape,
                    infringee_shape,
                    primitive_node,
                ))
        })
    }

    pub(super) fn locate_possible_infringees(
        &self,
        node: PrimitiveIndex,
    ) -> impl Iterator<Item = GenericNode<PrimitiveIndex, GenericIndex<CW>>> + '_ {
        let limiting_shape = node.primitive_ref(self).shape().inflate(
            node.primitive_ref(self)
                .maybe_net()
                .map(|net| self.rules().largest_clearance(Some(net)))
                .unwrap_or(0.0),
        );

        self.recording_geometry_with_rtree()
            .rtree()
            .locate_in_envelope_intersecting(
                &limiting_shape.envelope_3d(0.0, node.primitive_ref(self).layer()),
            )
            .map(|wrapper| wrapper.data)
    }

    pub(super) fn detect_collision_except(
        &self,
        collider: PrimitiveIndex,
        predicate: &impl Fn(&Self, PrimitiveIndex, PrimitiveIndex) -> bool,
    ) -> Option<Collision> {
        let collider_shape = collider.primitive_ref(self).shape();

        self.recording_geometry_with_rtree()
            .rtree()
            .locate_in_envelope_intersecting(&collider_shape.full_height_envelope_3d(0.0, 2))
            .filter_map(|wrapper| {
                if let GenericNode::Primitive(collidee) = wrapper.data {
                    Some(collidee)
                } else {
                    None
                }
            })
            // NOTE: Collisions can happen between two same-net loose
            // segs, so these cases in particular are not filtered out
            // here, unlike what is done in infringement code.
            .filter(|&collidee| collider != collidee)
            .filter(|&collidee| {
                !self.are_connectable(collider, collidee)
                    || ((matches!(collider, PrimitiveIndex::LoneLooseSeg(..))
                        || matches!(collider, PrimitiveIndex::SeqLooseSeg(..)))
                        && (matches!(collidee, PrimitiveIndex::LoneLooseSeg(..))
                            || matches!(collidee, PrimitiveIndex::SeqLooseSeg(..))))
            })
            .filter(|collidee| predicate(&self, collider, *collidee))
            .find_map(|collidee| {
                if collider.primitive_ref(self).layer() != collidee.primitive_ref(self).layer() {
                    return None;
                }

                let collidee_shape = collidee.primitive_ref(self).shape();

                if collider_shape.intersects(&collidee_shape) {
                    Some(Collision(collider_shape, collidee_shape, collidee))
                } else {
                    None
                }
            })
    }

    fn are_connectable(&self, node1: PrimitiveIndex, node2: PrimitiveIndex) -> bool {
        if let (Some(node1_net), Some(node2_net)) = (
            node1.primitive_ref(self).maybe_net(),
            node2.primitive_ref(self).maybe_net(),
        ) {
            node1_net == node2_net
        } else {
            true
        }
    }
}
