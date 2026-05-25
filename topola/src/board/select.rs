// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{
        Board,
        selections::{ComponentSelection, ComponentSelector, PinSelection, PinSelector},
    },
    primitives::{JointId, PolygonId, SegmentId, ViaId},
};

impl Board {
    pub fn pins_to_components(&mut self, pin_selection: PinSelection) -> ComponentSelection {
        let mut component_selection = ComponentSelection::new();

        for selector in pin_selection.0 {
            let Some(pin_id) = self.pin_id(&selector.pin) else {
                continue;
            };

            let Some(layer_id) = self.layer_id(&selector.layer) else {
                continue;
            };

            for joint_id in self.layout.layer_joints(layer_id) {
                if self.layout.joint(joint_id).spec.pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.joint_component_selector(joint_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }

            for via_id in self.layout.layer_vias(layer_id) {
                if self.layout.via(via_id).spec.pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.via_component_selector(via_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }

            for segment_id in self.layout.layer_segments(layer_id) {
                if self.layout.segment(segment_id).spec.pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.segment_component_selector(segment_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }

            for polygon_id in self.layout.layer_polygons(layer_id) {
                if self.layout.polygon(polygon_id).pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.polygon_component_selector(polygon_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }
        }

        component_selection
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

    pub fn joint_component_selector(&self, id: JointId) -> Option<ComponentSelector> {
        let joint = self.layout.joint(id);

        Some(ComponentSelector {
            component: self.component_name(joint.spec.component?)?.to_string(),
        })
    }

    pub fn segment_component_selector(&self, id: SegmentId) -> Option<ComponentSelector> {
        let segment = self.layout.segment(id);

        Some(ComponentSelector {
            component: self.component_name(segment.spec.component?)?.to_string(),
        })
    }

    pub fn via_component_selector(&self, id: ViaId) -> Option<ComponentSelector> {
        let via = self.layout.via(id);

        Some(ComponentSelector {
            component: self.component_name(via.spec.component?)?.to_string(),
        })
    }

    pub fn polygon_component_selector(&self, id: PolygonId) -> Option<ComponentSelector> {
        let polygon = self.layout.polygon(id);

        Some(ComponentSelector {
            component: self.component_name(polygon.component?)?.to_string(),
        })
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

    pub fn joint_pin_selector(&self, id: JointId) -> Option<PinSelector> {
        let joint = self.layout.joint(id);

        Some(PinSelector {
            pin: self.pin_name(joint.spec.pin?)?.to_string(),
            layer: self.layer_name(joint.spec.layer)?,
        })
    }

    pub fn segment_pin_selector(&self, id: SegmentId) -> Option<PinSelector> {
        let segment = self.layout.segment(id);

        Some(PinSelector {
            pin: self.pin_name(segment.spec.pin?)?.to_string(),
            layer: self.layer_name(segment.layer)?,
        })
    }

    pub fn via_pin_selector(&self, id: ViaId) -> Option<PinSelector> {
        let via = self.layout.via(id);

        Some(PinSelector {
            pin: self.pin_name(via.spec.pin?)?.to_string(),
            layer: self.layer_name(via.min_layer)?,
        })
    }

    pub fn polygon_pin_selector(&self, id: PolygonId) -> Option<PinSelector> {
        let polygon = self.layout.polygon(id);

        Some(PinSelector {
            pin: self.pin_name(polygon.pin?)?.to_string(),
            layer: self.layer_name(polygon.layer)?,
        })
    }
}
