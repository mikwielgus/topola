// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use derive_getters::{Dissolve, Getters};
use derive_more::Constructor;
use rstar::{
    AABB, RTree,
    primitives::{GeomWithData, Rectangle},
};
use serde::{Deserialize, Serialize};
use stable_vec::StableVec;
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::{
    Joint, JointId, Polygon, PolygonId, Segment, SegmentId, Vector2, Via, ViaId,
    primitives::{SegmentSpec, ViaSpec},
};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinId(usize);

impl PinId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
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
    pub fn index(self) -> usize {
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
            self.pins[pin_id.index()].joints.push(joint_id);
        }

        joint_id
    }

    pub fn add_segment(&mut self, spec: SegmentSpec) -> SegmentId {
        self.add_segment_raw(Segment {
            spec,
            endpoints: [
                self.joint(spec.endjoints[0]).position,
                self.joint(spec.endjoints[1]).position,
            ],
            layer: self.joint(spec.endjoints[0]).layer,
            net: self.joint(spec.endjoints[0]).net,
        })
    }

    pub fn add_segment_raw(&mut self, segment: Segment) -> SegmentId {
        let pin_id = segment.spec.pin;
        let bbox = segment.bbox();
        let segment_id = SegmentId::new(self.segments.push(segment));

        self.segments_rtree
            .insert(GeomWithData::new(bbox, segment_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.index()].segments.push(segment_id);
        }

        segment_id
    }

    pub fn add_via(&mut self, spec: ViaSpec) -> ViaId {
        let joint0 = self.joint(spec.endjoints[0]);
        let joint1 = self.joint(spec.endjoints[1]);

        self.add_via_raw(Via {
            spec,
            min_layer: std::cmp::min(joint0.layer, joint1.layer),
            max_layer: std::cmp::max(joint0.layer, joint1.layer),
            net: joint0.net,
            position: (joint0.position + joint1.position) / 2,
        })
    }

    pub fn add_via_raw(&mut self, via: Via) -> ViaId {
        let bbox = via.bbox();
        let pin_id = via.spec.pin;
        let via_id = ViaId::new(self.vias.push(via));

        self.vias_rtree.insert(GeomWithData::new(bbox, via_id), ());

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.index()].vias.push(via_id);
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
            self.pins[pin_id.index()].polygons.push(polygon_id);
        }

        polygon_id
    }

    pub fn locate_joints_at_point(
        &self,
        layer: usize,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |joint_id| {
                self.joints
                    .get(&joint_id.index())
                    .unwrap()
                    .contains_point(point)
            })
    }

    pub fn locate_segments_at_point(
        &self,
        layer: usize,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).contains_point(point))
    }

    // TODO: vias.

    pub fn locate_polygons_at_point(
        &self,
        layer: usize,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |polygon_id| {
                self.polygons
                    .get(&polygon_id.index())
                    .unwrap()
                    .contains_point(point)
            })
    }

    pub fn layer_joints(&self, layer: usize) -> impl Iterator<Item = JointId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.joint(id).layer == layer)
    }

    pub fn layer_segments(&self, layer: usize) -> impl Iterator<Item = SegmentId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.segment(id).layer == layer)
    }

    pub fn layer_polygons(&self, layer: usize) -> impl Iterator<Item = PolygonId> + '_ {
        let envelope = Self::whole_layer_aabb(layer);
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&id| self.polygon(id).layer == layer)
    }

    fn whole_layer_aabb(layer: usize) -> AABB<[i64; 3]> {
        AABB::from_corners(
            [i64::MIN, i64::MIN, layer as i64],
            [i64::MAX, i64::MAX, layer as i64],
        )
    }

    pub fn joint(&self, joint_id: JointId) -> &Joint {
        self.joints.get(&joint_id.index()).unwrap()
    }

    pub fn segment(&self, segment_id: SegmentId) -> &Segment {
        self.segments.get(&segment_id.index()).unwrap()
    }

    pub fn polygon(&self, polygon_id: PolygonId) -> &Polygon {
        self.polygons.get(&polygon_id.index()).unwrap()
    }

    pub fn pin(&self, pin_id: PinId) -> &Pin {
        &self.pins[pin_id.index()]
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
