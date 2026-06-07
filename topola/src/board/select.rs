// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{
        Board,
        selections::{
            ComponentSelection, ComponentSelector, NetSelector, PinSelection, PinSelector,
        },
    },
    layout::primitives::{JointId, PolyId, SegId, ViaId},
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

            for seg_id in self.layout.layer_segs(layer_id) {
                if self.layout.seg(seg_id).spec.pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.seg_component_selector(seg_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }

            for poly_id in self.layout.layer_polys(layer_id) {
                if self.layout.poly(poly_id).spec.pin != Some(pin_id) {
                    continue;
                }

                let Some(component_selector) = self.poly_component_selector(poly_id) else {
                    continue;
                };

                component_selection.0.insert(component_selector);
            }
        }

        component_selection
    }

    pub fn joint_component_selector(&self, id: JointId) -> Option<ComponentSelector> {
        let joint = self.layout.joint(id);

        Some(ComponentSelector {
            component: self.component_name(joint.spec.component?)?.to_string(),
        })
    }

    pub fn seg_component_selector(&self, id: SegId) -> Option<ComponentSelector> {
        let seg = self.layout.seg(id);

        Some(ComponentSelector {
            component: self.component_name(seg.spec.component?)?.to_string(),
        })
    }

    pub fn via_component_selector(&self, id: ViaId) -> Option<ComponentSelector> {
        let via = self.layout.via(id);

        Some(ComponentSelector {
            component: self.component_name(via.spec.component?)?.to_string(),
        })
    }

    pub fn poly_component_selector(&self, id: PolyId) -> Option<ComponentSelector> {
        let poly = self.layout.poly(id);

        Some(ComponentSelector {
            component: self.component_name(poly.spec.component?)?.to_string(),
        })
    }

    pub fn joint_net_selector(&self, id: JointId) -> Option<NetSelector> {
        let joint = self.layout.joint(id);

        Some(NetSelector {
            net: self.net_name(joint.spec.net?)?.to_string(),
        })
    }

    pub fn seg_net_selector(&self, id: SegId) -> Option<NetSelector> {
        let seg = self.layout.seg(id);

        Some(NetSelector {
            net: self.net_name(seg.net?)?.to_string(),
        })
    }

    pub fn via_net_selector(&self, id: ViaId) -> Option<NetSelector> {
        let via = self.layout.via(id);

        Some(NetSelector {
            net: self.net_name(via.net?)?.to_string(),
        })
    }

    pub fn poly_net_selector(&self, id: PolyId) -> Option<NetSelector> {
        let poly = self.layout.poly(id);

        Some(NetSelector {
            net: self.net_name(poly.spec.net?)?.to_string(),
        })
    }

    pub fn joint_pin_selector(&self, id: JointId) -> Option<PinSelector> {
        let joint = self.layout.joint(id);

        Some(PinSelector {
            pin: self.pin_name(joint.spec.pin?)?.to_string(),
            layer: self.layer_name(joint.spec.layer)?,
        })
    }

    pub fn seg_pin_selector(&self, id: SegId) -> Option<PinSelector> {
        let seg = self.layout.seg(id);

        Some(PinSelector {
            pin: self.pin_name(seg.spec.pin?)?.to_string(),
            layer: self.layer_name(seg.layer)?,
        })
    }

    pub fn via_pin_selector(&self, id: ViaId) -> Option<PinSelector> {
        let via = self.layout.via(id);

        Some(PinSelector {
            pin: self.pin_name(via.spec.pin?)?.to_string(),
            layer: self.layer_name(via.min_layer)?,
        })
    }

    pub fn poly_pin_selector(&self, id: PolyId) -> Option<PinSelector> {
        let poly = self.layout.poly(id);

        Some(PinSelector {
            pin: self.pin_name(poly.spec.pin?)?.to_string(),
            layer: self.layer_name(poly.spec.layer)?,
        })
    }
}
