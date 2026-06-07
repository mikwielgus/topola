// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::layout::{
    Layout,
    primitives::{JointId, PolygonId, SegmentId, ViaId},
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

    pub fn delete_segment(&mut self, segment_id: SegmentId) {
        let segment = self.segment(segment_id);
        let bbox = segment.bbox();
        let component = segment.spec.component;
        let pin = segment.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component
                    .segments
                    .iter()
                    .position(|&curr| curr == segment_id)
                {
                    component.segments.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.segments.iter().position(|&curr| curr == segment_id) {
                    pin.segments.remove(index);
                }
            });
        }

        self.segments_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), segment_id));
        self.segments.remove(&segment_id.index());
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

    pub fn delete_polygon(&mut self, polygon_id: PolygonId) {
        let polygon = self.polygon(polygon_id);
        let bbox = polygon.bbox();
        let component = polygon.spec.component;
        let pin = polygon.spec.pin;

        if let Some(component_id) = component {
            self.components.modify(component_id.index(), |component| {
                if let Some(index) = component
                    .polygons
                    .iter()
                    .position(|&curr| curr == polygon_id)
                {
                    component.polygons.remove(index);
                }
            });
        }

        if let Some(pin_id) = pin {
            self.pins.modify(pin_id.index(), |pin| {
                if let Some(index) = pin.polygons.iter().position(|&curr| curr == polygon_id) {
                    pin.polygons.remove(index);
                }
            });
        }

        self.polygons_rtree
            .remove(&GeomWithData::new(bbox.rtree_rectangle(), polygon_id));
        self.polygons.remove(&polygon_id.index());
    }
}
