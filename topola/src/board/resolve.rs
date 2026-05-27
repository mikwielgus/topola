// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{Board, selections::ComponentSelection},
    layout::compounds::ComponentId,
    primitives::{JointId, PolygonId, SegmentId, ViaId},
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

    pub fn resolve_net_segments(&self, selection: NetSelection) -> impl Iterator<Item = SegmentId> {
        let mut resolved_segments = Vec::new();

        for (index, _) in self.layout.segments().container() {
            let segment_id = SegmentId::new(index);

            let Some(selector) = self.segment_net_selector(segment_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_segments.push(segment_id);
            }
        }

        resolved_segments.into_iter()
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

    pub fn resolve_net_polygons(&self, selection: NetSelection) -> impl Iterator<Item = PolygonId> {
        let mut resolved_polygons = Vec::new();

        for (index, _) in self.layout.polygons().container() {
            let polygon_id = PolygonId::new(index);

            let Some(selector) = self.polygon_net_selector(polygon_id) else {
                continue;
            };

            if selection.0.contains(&selector) {
                resolved_polygons.push(polygon_id);
            }
        }

        resolved_polygons.into_iter()
    }
}
