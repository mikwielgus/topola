// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board,
    layout::primitives::{JointId, PolyId, SegId, ViaId},
    selections::{ComponentSelection, NetSelection, NetSelector, PinSelection},
    vector::Vector2,
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

    pub fn components_contain_seg(
        &self,
        selection: &ComponentSelection,
        id: SegId,
    ) -> bool {
        let Some(selector) = self.seg_component_selector(id) else {
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

    pub fn components_contain_poly(
        &self,
        selection: &ComponentSelection,
        id: PolyId,
    ) -> bool {
        let Some(selector) = self.poly_component_selector(id) else {
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

    pub fn nets_contain_seg(&self, selection: &NetSelection, id: SegId) -> bool {
        let seg = self.layout.seg(id);
        let Some(net_name) = seg.net.and_then(|net| self.net_name(net)) else {
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

    pub fn nets_contain_poly(&self, selection: &NetSelection, id: PolyId) -> bool {
        let poly = self.layout.poly(id);
        let Some(net_name) = poly.spec.net.and_then(|net| self.net_name(net)) else {
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

    pub fn pins_contain_seg(&self, selection: &PinSelection, id: SegId) -> bool {
        let Some(selector) = self.seg_pin_selector(id) else {
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

    pub fn pins_contain_poly(&self, selection: &PinSelection, id: PolyId) -> bool {
        let Some(selector) = self.poly_pin_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }
}
