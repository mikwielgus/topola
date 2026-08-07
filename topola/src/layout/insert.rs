// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::layout::{
    Layout,
    compounds::{Component, ComponentId, Pin, PinId, PinSpec},
    primitives::{
        Joint, JointId, JointSpec, Poly, PolyId, Seg, SegId, SegSpec, Via, ViaId, ViaSpec,
    },
};

impl Layout {
    pub fn insert_component(&mut self) -> ComponentId {
        ComponentId::new(self.components.push(Component::new()))
    }

    pub fn insert_pin(&mut self, spec: PinSpec) -> PinId {
        let pin_id = PinId::new(self.pins.push(Pin::new(spec)));

        if let Some(component_id) = spec.component {
            self.components.modify(component_id.index(), |component| {
                component.pins.push(pin_id)
            });
        }

        if let Some(net_id) = spec.net {
            self.nets
                .modify(net_id.index(), |net| net.pins.push(pin_id));
        }

        pin_id
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        let joint = Joint::new(spec);
        let bbox = joint.bbox();
        let component_id = joint.spec.component;
        let pin_id = joint.spec.pin;
        let joint_id = JointId::new(self.joints.push(joint));

        self.joints_rtree
            .insert(GeomWithData::new(bbox.rtree_rectangle(), joint_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.joints.push(joint_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.joints.push(joint_id));
        }

        joint_id
    }

    pub fn insert_seg(&mut self, spec: SegSpec) -> SegId {
        let endjoints = [
            &self.joints[spec.endjoints[0].index()],
            &self.joints[spec.endjoints[1].index()],
        ];
        self.insert_seg_raw(Seg::new(spec, endjoints))
    }

    pub fn insert_seg_raw(&mut self, seg: Seg) -> SegId {
        let component_id = seg.spec.component;
        let pin_id = seg.spec.pin;
        let bbox = seg.bbox();
        let seg_id = SegId::new(self.segs.push(seg));

        self.joints.modify(seg.spec.endjoints[0].index(), |joint| {
            joint.segs.push(seg_id)
        });
        self.joints.modify(seg.spec.endjoints[1].index(), |joint| {
            joint.segs.push(seg_id)
        });

        self.segs_rtree
            .insert(GeomWithData::new(bbox.rtree_rectangle(), seg_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.segs.push(seg_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.segs.push(seg_id));
        }

        seg_id
    }

    pub fn insert_via(&mut self, spec: ViaSpec) -> ViaId {
        let endjoints = [
            &self.joints[spec.endjoints[0].index()],
            &self.joints[spec.endjoints[1].index()],
        ];
        self.insert_via_raw(Via::new(spec, endjoints))
    }

    pub fn insert_via_raw(&mut self, via: Via) -> ViaId {
        let bbox = via.bbox();
        let component_id = via.spec.component;
        let pin_id = via.spec.pin;
        let via_id = ViaId::new(self.vias.push(via));

        self.joints.modify(via.spec.endjoints[0].index(), |joint| {
            joint.vias.push(via_id)
        });
        self.joints.modify(via.spec.endjoints[1].index(), |joint| {
            joint.vias.push(via_id)
        });

        self.vias_rtree
            .insert(GeomWithData::new(bbox.rtree_rectangle(), via_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.vias.push(via_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.vias.push(via_id));
        }

        via_id
    }

    pub fn insert_poly(&mut self, poly: Poly) -> PolyId {
        let bbox = poly.bbox();
        let component_id = poly.spec.component;
        let pin_id = poly.spec.pin;
        let poly_id = PolyId::new(self.polys.push(poly));

        self.polys_rtree
            .insert(GeomWithData::new(bbox.rtree_rectangle(), poly_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.polys.push(poly_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.polys.push(poly_id));
        }

        poly_id
    }
}
