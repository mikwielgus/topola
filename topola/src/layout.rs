// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use derive_getters::{Dissolve, Getters};
use stable_vec::StableVec;
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PinId(usize);

impl PinId {
    /// Wrap a pin index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NetId(usize);

impl NetId {
    /// Wrap a joint index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JointId(usize);

impl JointId {
    /// Wrap a joint index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub position: [i64; 2],
    pub layer: usize,
    pub radius: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SegmentId(usize);

impl SegmentId {
    /// Wrap a segment index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub endjoints: [JointId; 2],
    pub layer: usize,
    pub half_width: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ViaId(usize);

impl ViaId {
    /// Wrap a via index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Via {
    pub endpoints: [JointId; 2],
    pub layer: usize,
    pub radius: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PolygonId(usize);

impl PolygonId {
    /// Wrap a polygon index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Polygon {
    pub vertices: Vec<[i64; 2]>,
    pub layer: usize,
    pub net: NetId,
    pub pin: Option<PinId>,
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
        }
    }

    pub fn add_pin(&mut self) -> PinId {
        PinId::new(self.pins.push(Pin::new()))
    }

    pub fn add_joint(&mut self, joint: Joint) -> JointId {
        let pin_id = joint.pin;
        let joint_id = JointId::new(self.joints.push(joint));

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].joints.push(joint_id);
        }

        joint_id
    }

    pub fn add_segment(&mut self, segment: Segment) -> SegmentId {
        let pin_id = segment.pin;
        let segment_id = SegmentId::new(self.segments.push(segment));

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].segments.push(segment_id);
        }

        segment_id
    }

    pub fn add_via(&mut self, via: Via) -> ViaId {
        let pin_id = via.pin;
        let via_id = ViaId::new(self.vias.push(via));

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].vias.push(via_id);
        }

        via_id
    }

    pub fn add_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let pin_id = polygon.pin;
        let polygon_id = PolygonId::new(self.polygons.push(polygon));

        if let Some(pin_id) = pin_id {
            self.pins[pin_id.id()].polygons.push(polygon_id);
        }

        polygon_id
    }

    pub fn segment_endpoints(&self, segment: SegmentId) -> [[i64; 2]; 2] {
        let endjoints = self.segments.get(&segment.id()).unwrap().endjoints;
        [
            self.joints.get(&endjoints[0].id()).unwrap().position,
            self.joints.get(&endjoints[1].id()).unwrap().position,
        ]
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
    }
}

impl FlushDelta<LayoutHalfDelta> for Layout {
    fn flush_delta(&mut self) -> Delta<LayoutHalfDelta> {
        let (removed_joints, inserted_joints) = self.joints.flush_delta().dissolve();
        let (removed_segments, inserted_segments) = self.segments.flush_delta().dissolve();
        let (removed_vias, inserted_vias) = self.vias.flush_delta().dissolve();
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();

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
