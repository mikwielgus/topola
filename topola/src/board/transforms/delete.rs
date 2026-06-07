// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{board::Board, selections::NetSelection};

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

        for seg_id in self
            .resolve_net_segs(selection.clone())
            .filter(|&seg_id| self.layout.seg(seg_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_seg(seg_id);
        }

        for via_id in self
            .resolve_net_vias(selection.clone())
            .filter(|&via_id| self.layout.via(via_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_via(via_id);
        }

        for poly_id in self
            .resolve_net_polys(selection.clone())
            .filter(|&poly_id| self.layout.poly(poly_id).spec.pin.is_none())
            .collect::<Vec<_>>()
            .clone()
        {
            self.layout.delete_poly(poly_id);
        }
    }
}
