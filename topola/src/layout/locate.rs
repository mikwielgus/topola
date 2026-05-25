// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Vector2,
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
}
