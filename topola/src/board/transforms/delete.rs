// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{Board, selections::NetSelection};

impl Board {
    pub fn delete_net_free_primitives(&mut self, selection: NetSelection) {
        for joint_id in self
            .resolve_net_joints(selection.clone())
            .filter(|&joint_id| self.layout.joint(joint_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_joint(joint_id);
        }

        for segment_id in self
            .resolve_net_segments(selection.clone())
            .filter(|&segment_id| self.layout.segment(segment_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_segment(segment_id);
        }

        for via_id in self
            .resolve_net_vias(selection.clone())
            .filter(|&via_id| self.layout.via(via_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_via(via_id);
        }

        for polygon_id in self
            .resolve_net_polygons(selection.clone())
            .filter(|&polygon_id| self.layout.polygon(polygon_id).pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_polygon(polygon_id);
        }
    }
}
