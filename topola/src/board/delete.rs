// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board,
    primitives::{JointId, PolygonId, SegmentId, ViaId},
};

impl Board {
    pub fn delete_joint(&mut self, joint_id: JointId) {
        self.layout.delete_joint(joint_id);
    }

    pub fn delete_segment(&mut self, segment_id: SegmentId) {
        self.layout.delete_segment(segment_id);
    }

    pub fn delete_via(&mut self, via_id: ViaId) {
        self.layout.delete_via(via_id);
    }

    pub fn delete_polygon(&mut self, polygon_id: PolygonId) {
        self.layout.delete_polygon(polygon_id);
    }
}
