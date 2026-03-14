// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use derive_getters::{Dissolve, Getters};
use derive_more::Constructor;
use rstar::{
    RTree,
    primitives::{GeomWithData, Rectangle},
};
use serde::{Deserialize, Serialize};
use stable_vec::StableVec;
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::{Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Via, ViaId, Vector2};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinId(usize);

impl PinId {
    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Pin {
    joints: Vec<JointId>,
    segments: Vec<SegmentId>,
    vias: Vec<ViaId>,
    polygons: Vec<PolygonId>,
}

impl Pin {
    pub fn new() -> Self {
        Self {
            joints: Vec::new(),
            segments: Vec::new(),
            vias: Vec::new(),
            polygons: Vec::new(),
        }
    }
}

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct NetId(usize);

impl NetId {
    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Getters)]
pub struct Layout {
    boundary: Vec<[i64; 2]>,
    place_boundary: Vec<[i64; 2]>,
    layer_count: usize,

    pins: StableVec<Pin>,

    joints: Recorder<StableVec<Joint>>,
    segments: Recorder<StableVec<Segment>>,
    vias: Recorder<StableVec<Via>>,
    polygons: Recorder<StableVec<Polygon>>,

    joints_rtree: Recorder<RTree<GeomWithData<Rectangle<[i64; 3]>, JointId>>>,
    segments_rtree: Recorder<RTree<GeomWithData<Rectangle<[i64; 3]>, SegmentId>>>,
    vias_rtree: Recorder<RTree<GeomWithData<Rectangle<[i64; 3]>, ViaId>>>,
    polygons_rtree: Recorder<RTree<GeomWithData<Rectangle<[i64; 3]>, PolygonId>>>,
}

impl Layout {
    pub fn new(boundary: Vec<[i64; 2]>, layer_count: usize) -> Self {
        Self {
            boundary: boundary.clone(),
            place_boundary: boundary,
            layer_count,

            pins: StableVec::new(),

            joints: Recorder::new(StableVec::new()),
            segments: Recorder::new(StableVec::new()),
            vias: Recorder::new(StableVec::new()),
            polygons: Recorder::new(StableVec::new()),

            joints_rtree: Recorder::new(RTree::new()),
            segments_rtree: Recorder::new(RTree::new()),
            vias_rtree: Recorder::new(RTree::new()),
            polygons_rtree: Recorder::new(RTree::new()),
        }
    }

    pub fn add_pin(&mut self) -> PinId {
        PinId::new(self.pins.push(Pin::new()))
    }

    pub fn add_joint(&mut self, joint: Joint) -> JointId {
        let bbox = joint.bbox();
        let pin_id = joint.pin;
        let joint_id = JointId::new(self.joints.push(joint));

        self.joints_rtree
            .insert(GeomWithData::new(bbox, joint_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].joints.push(joint_id);
        }

        joint_id
    }

    pub fn add_segment(&mut self, segment: Segment) -> SegmentId {
        let pin_id = segment.pin;
        let segment_id = SegmentId::new(self.segments.push(segment));
        let bbox = self.segment_bbox(segment_id);

        self.segments_rtree
            .insert(GeomWithData::new(bbox, segment_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].segments.push(segment_id);
        }

        segment_id
    }

    pub fn add_via(&mut self, via: Via) -> ViaId {
        //let bbox = via.bbox();
        let pin_id = via.pin;
        let via_id = ViaId::new(self.vias.push(via));

        //self.vias_rtree.insert(GeomWithData::new(bbox, via_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].vias.push(via_id);
        }

        via_id
    }

    pub fn add_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let bbox = polygon.bbox();
        let pin_id = polygon.pin;
        let polygon_id = PolygonId::new(self.polygons.push(polygon));

        self.polygons_rtree
            .insert(GeomWithData::new(bbox, polygon_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].polygons.push(polygon_id);
        }

        polygon_id
    }

    pub fn segment_endpoints(&self, segment_id: SegmentId) -> [Vector2<i64>; 2] {
        let endjoints = self.segments.get(&segment_id.id()).unwrap().endjoints;
        [
            self.joints.get(&endjoints[0].id()).unwrap().position,
            self.joints.get(&endjoints[1].id()).unwrap().position,
        ]
    }

    pub fn segment_bbox(&self, segment_id: SegmentId) -> Rectangle<[i64; 3]> {
        let endpoints = self.segment_endpoints(segment_id);
        let layer = self.segments.get(&segment_id.id()).unwrap().layer as i64;
        let half_width = self.segments.get(&segment_id.id()).unwrap().half_width as i64;

        let min_x = std::cmp::min(endpoints[0].x, endpoints[1].x) - half_width;
        let min_y = std::cmp::min(endpoints[0].y, endpoints[1].y) - half_width;
        let max_x = std::cmp::max(endpoints[0].x, endpoints[1].x) + half_width;
        let max_y = std::cmp::max(endpoints[0].y, endpoints[1].y) + half_width;

        Rectangle::from_corners([min_x, min_y, layer], [max_x, max_y, layer])
    }

    pub fn locate_joints_at_point(
        &self,
        layer: usize,
        point: [i64; 2],
    ) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point[0], point[1], layer as i64])
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_segments_at_point(
        &self,
        layer: usize,
        point: [i64; 2],
    ) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point[0], point[1], layer as i64])
            .map(|geom_with_data| geom_with_data.data)
    }

    // TODO: vias.

    pub fn locate_polygons_at_point(
        &self,
        layer: usize,
        point: [i64; 2],
    ) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point[0], point[1], layer as i64])
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn pin(&self, pin: PinId) -> &Pin {
        &self.pins[pin.id()]
    }
}

#[derive(Clone, Debug, Dissolve)]
pub struct LayoutHalfDelta {
    joints: BTreeMap<usize, Joint>,
    segments: BTreeMap<usize, Segment>,
    vias: BTreeMap<usize, Via>,
    polygons: BTreeMap<usize, Polygon>,
}

impl ApplyDelta<LayoutHalfDelta> for Layout {
    fn apply_delta(&mut self, delta: &Delta<LayoutHalfDelta>) {
        let (removed, inserted) = delta.clone().dissolve();

        let joints_delta = Delta::with_removed_inserted(removed.joints, inserted.joints);
        self.joints.apply_delta(&joints_delta);

        let segments_delta = Delta::with_removed_inserted(removed.segments, inserted.segments);
        self.segments.apply_delta(&segments_delta);

        let vias_delta = Delta::with_removed_inserted(removed.vias, inserted.vias);
        self.vias.apply_delta(&vias_delta);

        let polygons_delta = Delta::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_delta(&polygons_delta);

        // TODO R-trees.
    }
}

impl FlushDelta<LayoutHalfDelta> for Layout {
    fn flush_delta(&mut self) -> Delta<LayoutHalfDelta> {
        let (removed_joints, inserted_joints) = self.joints.flush_delta().dissolve();
        let (removed_segments, inserted_segments) = self.segments.flush_delta().dissolve();
        let (removed_vias, inserted_vias) = self.vias.flush_delta().dissolve();
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();

        // TODO R-trees.

        Delta::with_removed_inserted(
            LayoutHalfDelta {
                joints: removed_joints,
                segments: removed_segments,
                vias: removed_vias,
                polygons: removed_polygons,
            },
            LayoutHalfDelta {
                joints: inserted_joints,
                segments: inserted_segments,
                vias: inserted_vias,
                polygons: inserted_polygons,
            },
        )
    }
}
