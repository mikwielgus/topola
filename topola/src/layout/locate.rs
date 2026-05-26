// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use crate::{
    Rect3, Vector3,
    layout::{LayerId, Layout, compounds::NetId},
    primitives::{JointId, PolygonId, SegmentId, ViaId},
};

impl Layout {
    pub fn locate_joints_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = JointId> {
        let point2 = point.xy();
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joints[joint_id.index()].contains_point(point2))
    }

    pub fn locate_joints_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = JointId> {
        let rect_aabb = rect.aabb3();
        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_joints_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = JointId> {
        let rect_aabb = rect.aabb3();
        self.joints_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_segments_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = SegmentId> {
        let point2 = point.xy();
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).contains_point(point2))
    }

    pub fn locate_segments_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let rect_aabb = rect.aabb3();
        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_segments_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = SegmentId> {
        let rect_aabb = rect.aabb3();
        self.segments_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_vias_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = ViaId> {
        let layer = LayerId::new(point.z as usize);
        let point2 = point.xy();
        self.vias_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| self.vias[via_id.index()].contains_point(layer, point2))
    }

    pub fn locate_vias_intersecting_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = ViaId> {
        let rect_aabb = rect.aabb3();
        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_vias_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = ViaId> {
        let rect_aabb = rect.aabb3();
        self.vias_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_polygons_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = PolygonId> {
        let point2 = point.xy();
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| self.polygons[polygon_id.index()].contains_point(point2))
    }

    pub fn locate_polygons_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let rect_aabb = rect.aabb3();
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_polygons_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = PolygonId> {
        let rect_aabb = rect.aabb3();
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_nets_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_at_point(point) {
            nets.insert(self.joint(joint_id).spec.net);
        }

        for segment_id in self.locate_segments_at_point(point) {
            nets.insert(self.segment(segment_id).net);
        }

        for via_id in self.locate_vias_at_point(point) {
            nets.insert(self.via(via_id).net);
        }

        for polygon_id in self.locate_polygons_at_point(point) {
            nets.insert(self.polygon(polygon_id).net);
        }

        nets.into_iter()
    }

    pub fn locate_nets_intersecting_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_intersecting_rect(rect) {
            nets.insert(self.joint(joint_id).spec.net);
        }

        for segment_id in self.locate_segments_intersecting_rect(rect) {
            nets.insert(self.segment(segment_id).net);
        }

        for via_id in self.locate_vias_intersecting_rect(rect) {
            nets.insert(self.via(via_id).net);
        }

        for polygon_id in self.locate_polygons_intersecting_rect(rect) {
            nets.insert(self.polygon(polygon_id).net);
        }

        nets.into_iter()
    }

    pub fn locate_nets_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_inside_rect(rect) {
            nets.insert(self.joint(joint_id).spec.net);
        }

        for segment_id in self.locate_segments_inside_rect(rect) {
            nets.insert(self.segment(segment_id).net);
        }

        for via_id in self.locate_vias_inside_rect(rect) {
            nets.insert(self.via(via_id).net);
        }

        for polygon_id in self.locate_polygons_inside_rect(rect) {
            nets.insert(self.polygon(polygon_id).net);
        }

        nets.into_iter()
    }
}
