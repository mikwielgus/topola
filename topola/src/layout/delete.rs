// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::layout::{
    Layout,
    primitives::{JointId, PolyId, SegId, ViaId},
};

impl Layout {
    pub fn delete_joint(&mut self, joint_id: JointId) {
        let joint = self.joint(joint_id);
        let bbox = joint.bbox();
        let component = joint.spec.component;
        let pin = joint.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component.joints.iter().position(|&curr| curr == joint_id) {
                    component.joints.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.joints.iter().position(|&curr| curr == joint_id) {
                    pin.joints.remove(index);
                }
            });
        }

        self.joints_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), joint_id));
        self.joints.remove(&joint_id.index());
    }

    pub fn delete_seg(&mut self, seg_id: SegId) {
        let seg = self.seg(seg_id);
        let bbox = seg.bbox();
        let component = seg.spec.component;
        let pin = seg.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component
                    .segs
                    .iter()
                    .position(|&curr| curr == seg_id)
                {
                    component.segs.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.segs.iter().position(|&curr| curr == seg_id) {
                    pin.segs.remove(index);
                }
            });
        }

        self.segs_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), seg_id));
        self.segs.remove(&seg_id.index());
    }

    pub fn delete_via(&mut self, via_id: ViaId) {
        let via = self.via(via_id);
        let bbox = via.bbox();
        let component = via.spec.component;
        let pin = via.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component.vias.iter().position(|&curr| curr == via_id) {
                    component.vias.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.vias.iter().position(|&curr| curr == via_id) {
                    pin.vias.remove(index);
                }
            });
        }

        self.vias_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), via_id));
        self.vias.remove(&via_id.index());
    }

    pub fn delete_poly(&mut self, poly_id: PolyId) {
        let poly = self.poly(poly_id);
        let bbox = poly.bbox();
        let component = poly.spec.component;
        let pin = poly.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component
                    .polys
                    .iter()
                    .position(|&curr| curr == poly_id)
                {
                    component.polys.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.polys.iter().position(|&curr| curr == poly_id) {
                    pin.polys.remove(index);
                }
            });
        }

        self.polys_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), poly_id));
        self.polys.remove(&poly_id.index());
    }
}
