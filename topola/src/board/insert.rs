// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board,
    layout::compounds::ComponentId,
    primitives::{
        JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
        ViaSpec,
    },
};

impl Board {
    pub fn insert_component(&mut self) -> ComponentId {
        self.layout.insert_component()
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        self.layout.insert_joint(spec)
    }

    pub fn insert_segment(&mut self, spec: SegmentSpec) -> SegmentId {
        self.layout.insert_segment(spec)
    }

    pub fn insert_segment_raw(&mut self, segment: Segment) -> SegmentId {
        self.layout.insert_segment_raw(segment)
    }

    pub fn insert_via(&mut self, spec: ViaSpec) -> ViaId {
        self.layout.insert_via(spec)
    }

    pub fn insert_via_raw(&mut self, via: Via) -> ViaId {
        self.layout.insert_via_raw(via)
    }

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        self.layout.insert_polygon(polygon)
    }
}
