// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use rstar::primitives::GeomWithData;

use crate::{
    Pin, PinId,
    layout::{
        Layout,
        compounds::{Component, ComponentId},
    },
    primitives::{
        Joint, JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
        ViaSpec,
    },
};

impl Layout {
    pub fn insert_component(&mut self) -> ComponentId {
        ComponentId::new(self.components.push(Component::new()))
    }

    pub fn insert_pin(&mut self) -> PinId {
        PinId::new(self.pins.push(Pin::new()))
    }

    pub fn insert_joint(&mut self, spec: JointSpec) -> JointId {
        let joint = Joint {
            spec,
            segments: Vec::new(),
            vias: Vec::new(),
        };
        let bbox = joint.bbox();
        let component_id = joint.spec.component;
        let pin_id = joint.spec.pin;
        let joint_id = JointId::new(self.joints.push(joint));

        self.joints_rtree
            .insert(GeomWithData::new(bbox, joint_id), ());

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

    pub fn insert_segment(&mut self, spec: SegmentSpec) -> SegmentId {
        self.insert_segment_raw(Segment {
            spec,
            endpoints: [
                self.joint(spec.endjoints[0]).spec.position,
                self.joint(spec.endjoints[1]).spec.position,
            ],
            layer: self.joint(spec.endjoints[0]).spec.layer,
            net: self.joint(spec.endjoints[0]).spec.net,
        })
    }

    pub fn insert_segment_raw(&mut self, segment: Segment) -> SegmentId {
        let component_id = segment.spec.component;
        let pin_id = segment.spec.pin;
        let bbox = segment.bbox();
        let segment_id = SegmentId::new(self.segments.push(segment));

        self.joints
            .modify(segment.spec.endjoints[0].index(), |joint| {
                joint.segments.push(segment_id)
            });
        self.joints
            .modify(segment.spec.endjoints[1].index(), |joint| {
                joint.segments.push(segment_id)
            });

        self.segments_rtree
            .insert(GeomWithData::new(bbox, segment_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.segments.push(segment_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.segments.push(segment_id));
        }

        segment_id
    }

    pub fn insert_via(&mut self, spec: ViaSpec) -> ViaId {
        let joints = [self.joint(spec.endjoints[0]), self.joint(spec.endjoints[1])];

        self.insert_via_raw(Via {
            spec,
            position: joints[0].spec.position,
            min_layer: std::cmp::min(joints[0].spec.layer, joints[1].spec.layer),
            max_layer: std::cmp::max(joints[0].spec.layer, joints[1].spec.layer),
            net: joints[0].spec.net,
        })
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

        self.vias_rtree.insert(GeomWithData::new(bbox, via_id), ());

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

    pub fn insert_polygon(&mut self, polygon: Polygon) -> PolygonId {
        let bbox = polygon.bbox();
        let component_id = polygon.component;
        let pin_id = polygon.pin;
        let polygon_id = PolygonId::new(self.polygons.push(polygon));

        self.polygons_rtree
            .insert(GeomWithData::new(bbox, polygon_id), ());

        if let Some(component_id) = component_id {
            self.components.modify(component_id.index(), |component| {
                component.polygons.push(polygon_id)
            });
        }

        if let Some(pin_id) = pin_id {
            self.pins
                .modify(pin_id.index(), |pin| pin.polygons.push(polygon_id));
        }

        polygon_id
    }
}
