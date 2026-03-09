// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use derive_getters::Dissolve;
use stable_vec::StableVec;
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JointId(usize);

impl JointId {
    /// Wrap a vertex index in a newtype struct.
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
pub struct Joint {
    position: [i64; 2],
    radius: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SegmentId(usize);

impl SegmentId {
    /// Wrap a vertex index in a newtype struct.
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
pub struct Segment {
    endpoints: [JointId; 2],
    half_width: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArcId(usize);

impl ArcId {
    /// Wrap a vertex index in a newtype struct.
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
pub struct Arc {
    endpoints: [JointId; 2],
    focus: [i64; 2],
    half_width: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ViaId(usize);

impl ViaId {
    /// Wrap a vertex index in a newtype struct.
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
pub struct Via {
    endpoints: [JointId; 2],
    radius: u64,
}

#[derive(Clone, Debug)]
pub struct Layout {
    boundary: Vec<[i64; 2]>,
    place_boundary: Vec<[i64; 2]>,
    joints: Recorder<StableVec<Joint>>,
    segments: Recorder<StableVec<Segment>>,
    arcs: Recorder<StableVec<Arc>>,
    vias: Recorder<StableVec<Via>>,
}

impl Layout {
    pub fn new(boundary: Vec<[i64; 2]>) -> Self {
        Self {
            boundary: boundary.clone(),
            place_boundary: boundary,
            joints: Recorder::new(StableVec::new()),
            segments: Recorder::new(StableVec::new()),
            arcs: Recorder::new(StableVec::new()),
            vias: Recorder::new(StableVec::new()),
        }
    }

    pub fn add_joint(&mut self, joint: Joint) -> JointId {
        JointId::new(self.joints.push(joint))
    }

    pub fn add_segment(&mut self, segment: Segment) -> SegmentId {
        SegmentId::new(self.segments.push(segment))
    }

    pub fn add_arc(&mut self, arc: Arc) -> ArcId {
        ArcId::new(self.arcs.push(arc))
    }

    pub fn add_via(&mut self, via: Via) -> ViaId {
        ViaId::new(self.vias.push(via))
    }
}

#[derive(Clone, Debug, Dissolve)]
pub struct LayoutHalfDelta {
    joints: BTreeMap<usize, Joint>,
    segments: BTreeMap<usize, Segment>,
    arcs: BTreeMap<usize, Arc>,
    vias: BTreeMap<usize, Via>,
}

impl ApplyDelta<LayoutHalfDelta> for Layout {
    fn apply_delta(&mut self, delta: &Delta<LayoutHalfDelta>) {
        let (removed, inserted) = delta.clone().dissolve();

        let joints_delta = Delta::with_removed_inserted(removed.joints, inserted.joints);
        self.joints.apply_delta(&joints_delta);

        let segments_delta = Delta::with_removed_inserted(removed.segments, inserted.segments);
        self.segments.apply_delta(&segments_delta);

        let arcs_delta = Delta::with_removed_inserted(removed.arcs, inserted.arcs);
        self.arcs.apply_delta(&arcs_delta);

        let vias_delta = Delta::with_removed_inserted(removed.vias, inserted.vias);
        self.vias.apply_delta(&vias_delta);
    }
}

impl FlushDelta<LayoutHalfDelta> for Layout {
    fn flush_delta(&mut self) -> Delta<LayoutHalfDelta> {
        let (removed_joints, inserted_joints) = self.joints.flush_delta().dissolve();
        let (removed_segments, inserted_segments) = self.segments.flush_delta().dissolve();
        let (removed_arcs, inserted_arcs) = self.arcs.flush_delta().dissolve();
        let (removed_vias, inserted_vias) = self.vias.flush_delta().dissolve();

        Delta::with_removed_inserted(
            LayoutHalfDelta {
                joints: removed_joints,
                segments: removed_segments,
                arcs: removed_arcs,
                vias: removed_vias,
            },
            LayoutHalfDelta {
                joints: inserted_joints,
                segments: inserted_segments,
                arcs: inserted_arcs,
                vias: inserted_vias,
            },
        )
    }
}
