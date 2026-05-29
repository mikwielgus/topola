// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Board, Vector2,
    primitives::{JointId, PolygonId, SegmentId, ViaId},
    selections::{ComponentSelection, NetSelection, NetSelector, PinSelection},
};

impl Board {
    pub fn components_contain_point(
        &self,
        selection: &ComponentSelection,
        point: Vector2<i64>,
    ) -> bool {
        let Some(bbox) = self.components_bbox2(selection.clone()) else {
            return false;
        };

        bbox.contains_point(point)
    }

    pub fn components_contain_joint(&self, selection: &ComponentSelection, id: JointId) -> bool {
        let Some(selector) = self.joint_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn components_contain_segment(
        &self,
        selection: &ComponentSelection,
        id: SegmentId,
    ) -> bool {
        let Some(selector) = self.segment_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn components_contain_via(&self, selection: &ComponentSelection, id: ViaId) -> bool {
        let Some(selector) = self.via_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn components_contain_polygon(
        &self,
        selection: &ComponentSelection,
        id: PolygonId,
    ) -> bool {
        let Some(selector) = self.polygon_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn nets_contain_joint(&self, selection: &NetSelection, id: JointId) -> bool {
        let joint = self.layout.joint(id);
        let Some(net_name) = joint.spec.net.and_then(|net| self.net_name(net)) else {
            return false;
        };

        selection
            .0
            .contains(&NetSelector::new(net_name.to_string()))
    }

    pub fn nets_contain_segment(&self, selection: &NetSelection, id: SegmentId) -> bool {
        let segment = self.layout.segment(id);
        let Some(net_name) = segment.net.and_then(|net| self.net_name(net)) else {
            return false;
        };

        selection
            .0
            .contains(&NetSelector::new(net_name.to_string()))
    }

    pub fn nets_contain_via(&self, selection: &NetSelection, id: ViaId) -> bool {
        let via = self.layout.via(id);
        let Some(net_name) = via.net.and_then(|net| self.net_name(net)) else {
            return false;
        };

        selection
            .0
            .contains(&NetSelector::new(net_name.to_string()))
    }

    pub fn nets_contain_polygon(&self, selection: &NetSelection, id: PolygonId) -> bool {
        let polygon = self.layout.polygon(id);
        let Some(net_name) = polygon.net.and_then(|net| self.net_name(net)) else {
            return false;
        };

        selection
            .0
            .contains(&NetSelector::new(net_name.to_string()))
    }

    pub fn pins_contain_joint(&self, selection: &PinSelection, id: JointId) -> bool {
        let Some(selector) = self.joint_pin_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn pins_contain_segment(&self, selection: &PinSelection, id: SegmentId) -> bool {
        let Some(selector) = self.segment_pin_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn pins_contain_via(&self, selection: &PinSelection, id: ViaId) -> bool {
        let Some(selector) = self.via_pin_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn pins_contain_polygon(&self, selection: &PinSelection, id: PolygonId) -> bool {
        let Some(selector) = self.polygon_pin_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }
}
