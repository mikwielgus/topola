// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{Board, selections::ComponentSelection},
    layout::{
        compounds::ComponentId,
        primitives::{JointId, PolyId, SegId, ViaId},
    },
    selections::NetSelection,
};

impl Board {
    pub fn resolve_components(
        &self,
        selection: ComponentSelection,
    ) -> impl Iterator<Item = ComponentId> {
        selection
            .0
            .clone()
            .into_iter()
            .filter_map(|selector| self.component_id(&selector.component))
    }

    pub fn resolve_net_joints(&self, selection: NetSelection) -> impl Iterator<Item = JointId> {
        let mut resolved_joints = Vec::new();

        for (index, _) in self.layout.joints().container() {
            let joint_id = JointId::new(index);

            let Some(selector) = self.joint_net_selector(joint_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_joints.push(joint_id);
            }
        }

        resolved_joints.into_iter()
    }

    pub fn resolve_net_segs(&self, selection: NetSelection) -> impl Iterator<Item = SegId> {
        let mut resolved_segs = Vec::new();

        for (index, _) in self.layout.segs().container() {
            let seg_id = SegId::new(index);

            let Some(selector) = self.seg_net_selector(seg_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_segs.push(seg_id);
            }
        }

        resolved_segs.into_iter()
    }

    pub fn resolve_net_vias(&self, selection: NetSelection) -> impl Iterator<Item = ViaId> {
        let mut resolved_vias = Vec::new();

        for (index, _) in self.layout.vias().container() {
            let via_id = ViaId::new(index);

            let Some(selector) = self.via_net_selector(via_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_vias.push(via_id);
            }
        }

        resolved_vias.into_iter()
    }

    pub fn resolve_net_polys(&self, selection: NetSelection) -> impl Iterator<Item = PolyId> {
        let mut resolved_polys = Vec::new();

        for (index, _) in self.layout.polys().container() {
            let poly_id = PolyId::new(index);

            let Some(selector) = self.poly_net_selector(poly_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_polys.push(poly_id);
            }
        }

        resolved_polys.into_iter()
    }
}
