// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2, Vector2,
    layout::{LayerId, Layout},
    primitives::{JointId, PolygonId, SegmentId, ViaId},
};

impl Layout {
    pub fn locate_joints_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joints[joint_id.index()].contains_point(point))
    }

    pub fn locate_joints_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = JointId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joint(joint_id).spec.layer == layer)
    }

    pub fn locate_joints_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = JointId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.joints_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_segments_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).contains_point(point))
    }

    pub fn locate_segments_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).layer == layer)
    }

    pub fn locate_segments_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.segments_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_vias_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        self.vias_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| self.vias[via_id.index()].contains_point(layer, point))
    }

    pub fn locate_vias_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| {
                let via = self.via(via_id);
                via.min_layer <= layer && layer <= via.max_layer
            })
    }

    pub fn locate_vias_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.vias_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }

    pub fn locate_polygons_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, layer.index() as i64])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| self.polygons[polygon_id.index()].contains_point(point))
    }

    pub fn locate_polygons_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| self.polygon(polygon_id).layer == layer)
    }

    pub fn locate_polygons_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let rect_aabb = rect.aabb3(layer.index() as i64);
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope(&rect_aabb)
            .map(|geom_with_data| geom_with_data.data)
    }
}
