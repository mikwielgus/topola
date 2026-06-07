// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use crate::{
    layout::{
        Layout,
        compounds::NetId,
        primitives::{JointId, PolygonId, SegmentId, ViaId},
    },
    rect::{Rect2, Rect3},
    vector::{Vector2, Vector3},
};

impl Layout {
    pub fn locate_joints_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = JointId> {
        let at_point = self.locate_joints_at_point(point).collect::<Vec<_>>();

        let joints = if at_point.is_empty() {
            self.locate_joints_any_layer_at_point(point.xy()).collect()
        } else {
            at_point
        };

        joints.into_iter()
    }

    pub fn locate_joints_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joints[joint_id.index()].contains_point2(point.xy()))
    }

    pub fn locate_joints_any_layer_at_point(
        &self,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = JointId> {
        let envelope = point.z_extruded_infinitely().aabb();

        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| self.joints[joint_id.index()].contains_point2(point))
    }

    pub fn locate_joints_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = JointId> {
        let at_rect = self
            .locate_joints_intersecting_rect(rect)
            .collect::<Vec<_>>();

        let joints = if at_rect.is_empty() {
            self.locate_joints_any_layer_intersecting_rect(rect.xy())
                .collect()
        } else {
            at_rect
        };

        joints.into_iter()
    }

    pub fn locate_joints_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| {
                let joint = self.joint(joint_id);
                rect.xy()
                    .intersects_circle(joint.spec.position, joint.spec.radius as i64)
            })
    }

    pub fn locate_joints_any_layer_intersecting_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = JointId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.joints_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| {
                let joint = self.joint(joint_id);
                rect.intersects_circle(joint.spec.position, joint.spec.radius as i64)
            })
    }

    pub fn locate_joints_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = JointId> {
        let at_rect = self.locate_joints_inside_rect(rect).collect::<Vec<_>>();

        let joints = if at_rect.is_empty() {
            self.locate_joints_any_layer_inside_rect(rect.xy())
                .collect()
        } else {
            at_rect
        };

        joints.into_iter()
    }

    pub fn locate_joints_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = JointId> {
        self.joints_rtree
            .as_ref()
            .locate_in_envelope(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| {
                let joint = self.joint(joint_id);
                rect.xy()
                    .contains_circle(joint.spec.position, joint.spec.radius as i64)
            })
    }

    pub fn locate_joints_any_layer_inside_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = JointId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.joints_rtree
            .as_ref()
            .locate_in_envelope(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&joint_id| {
                let joint = self.joint(joint_id);
                rect.intersects_circle(joint.spec.position, joint.spec.radius as i64)
            })
    }

    pub fn locate_segments_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let at_point = self.locate_segments_at_point(point).collect::<Vec<_>>();

        let segments = if at_point.is_empty() {
            self.locate_segments_any_layer_at_point(point.xy())
                .collect()
        } else {
            at_point
        };

        segments.into_iter()
    }

    pub fn locate_segments_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segment(segment_id).contains_point2(point.xy()))
    }

    pub fn locate_segments_any_layer_at_point(
        &self,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let envelope = point.z_extruded_infinitely().aabb();

        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| self.segments[segment_id.index()].contains_point2(point))
    }

    pub fn locate_segments_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let at_point = self
            .locate_segments_intersecting_rect(rect)
            .collect::<Vec<_>>();

        let joints = if at_point.is_empty() {
            self.locate_segments_any_layer_intersecting_rect(rect.xy())
                .collect()
        } else {
            at_point
        };

        joints.into_iter()
    }

    pub fn locate_segments_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| {
                let segment = self.segment(segment_id);
                rect.xy().intersects_polygon(&segment.bounding_rectangle())
            })
    }

    pub fn locate_segments_any_layer_intersecting_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.segments_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| {
                let segment = self.segment(segment_id);
                rect.intersects_polygon(&segment.bounding_rectangle())
            })
    }

    pub fn locate_segments_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let at_point = self.locate_segments_inside_rect(rect).collect::<Vec<_>>();

        let joints = if at_point.is_empty() {
            self.locate_segments_any_layer_inside_rect(rect.xy())
                .collect()
        } else {
            at_point
        };

        joints.into_iter()
    }

    pub fn locate_segments_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = SegmentId> {
        self.segments_rtree
            .as_ref()
            .locate_in_envelope(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| {
                let segment = self.segment(segment_id);
                rect.xy().contains_polygon(&segment.bounding_rectangle())
            })
    }

    pub fn locate_segments_any_layer_inside_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = SegmentId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.segments_rtree
            .as_ref()
            .locate_in_envelope(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&segment_id| {
                let segment = self.segment(segment_id);
                rect.contains_polygon(&segment.bounding_rectangle())
            })
    }

    pub fn locate_vias_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let at_point = self.locate_vias_at_point(point).collect::<Vec<_>>();

        let vias = if at_point.is_empty() {
            self.locate_vias_any_layer_at_point(point.xy()).collect()
        } else {
            at_point
        };

        vias.into_iter()
    }

    pub fn locate_vias_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = ViaId> {
        self.vias_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| self.vias[via_id.index()].contains_point2(point.xy()))
    }

    pub fn locate_vias_any_layer_at_point(
        &self,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let envelope = point.z_extruded_infinitely().aabb();

        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| self.vias[via_id.index()].contains_point2(point))
    }

    pub fn locate_vias_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let at_point = self.locate_vias_intersecting_rect(rect).collect::<Vec<_>>();

        let vias = if at_point.is_empty() {
            self.locate_vias_any_layer_intersecting_rect(rect.xy())
                .collect()
        } else {
            at_point
        };

        vias.into_iter()
    }

    pub fn locate_vias_intersecting_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = ViaId> {
        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| {
                let via = self.via(via_id);
                rect.xy()
                    .intersects_circle(via.position, via.spec.radius as i64)
            })
    }

    pub fn locate_vias_any_layer_intersecting_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.vias_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| {
                let via = self.via(via_id);
                rect.intersects_circle(via.position, via.spec.radius as i64)
            })
    }

    pub fn locate_vias_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let at_rect = self.locate_vias_inside_rect(rect).collect::<Vec<_>>();

        let vias = if at_rect.is_empty() {
            self.locate_vias_any_layer_inside_rect(rect.xy()).collect()
        } else {
            at_rect
        };

        vias.into_iter()
    }

    pub fn locate_vias_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = ViaId> {
        self.vias_rtree
            .as_ref()
            .locate_in_envelope(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| {
                let via = self.via(via_id);
                rect.xy()
                    .contains_circle(via.position, via.spec.radius as i64)
            })
    }

    pub fn locate_vias_any_layer_inside_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ViaId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.vias_rtree
            .as_ref()
            .locate_in_envelope(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&via_id| {
                let via = self.via(via_id);
                rect.contains_circle(via.position, via.spec.radius as i64)
            })
    }

    pub fn locate_polygons_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let at_point = self.locate_polygons_at_point(point).collect::<Vec<_>>();

        let polygons = if at_point.is_empty() {
            self.locate_polygons_any_layer_at_point(point.xy())
                .collect()
        } else {
            at_point
        };

        polygons.into_iter()
    }

    pub fn locate_polygons_at_point(&self, point: Vector3<i64>) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_all_at_point(&[point.x, point.y, point.z])
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| {
                self.polygons[polygon_id.index()].contains_point2(point.xy())
            })
    }

    pub fn locate_polygons_any_layer_at_point(
        &self,
        point: Vector2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let envelope = point.z_extruded_infinitely().aabb();

        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| self.polygons[polygon_id.index()].contains_point2(point))
    }

    pub fn locate_polygons_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let at_rect = self
            .locate_polygons_intersecting_rect(rect)
            .collect::<Vec<_>>();

        let polygons = if at_rect.is_empty() {
            self.locate_polygons_any_layer_intersecting_rect(rect.xy())
                .collect()
        } else {
            at_rect
        };

        polygons.into_iter()
    }

    pub fn locate_polygons_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| {
                let polygon = self.polygon(polygon_id);
                rect.xy().intersects_polygon(&polygon.spec.vertices)
            })
    }

    pub fn locate_polygons_any_layer_intersecting_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.polygons_rtree
            .as_ref()
            .locate_in_envelope_intersecting(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| {
                let polygon = self.polygon(polygon_id);
                rect.intersects_polygon(&polygon.spec.vertices)
            })
    }

    pub fn locate_polygons_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let at_rect = self.locate_polygons_inside_rect(rect).collect::<Vec<_>>();

        let polygons = if at_rect.is_empty() {
            self.locate_polygons_any_layer_inside_rect(rect.xy())
                .collect()
        } else {
            at_rect
        };

        polygons.into_iter()
    }

    pub fn locate_polygons_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = PolygonId> {
        self.polygons_rtree
            .as_ref()
            .locate_in_envelope(&rect.aabb())
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| {
                let polygon = self.polygon(polygon_id);
                rect.xy().contains_polygon(&polygon.spec.vertices)
            })
    }

    pub fn locate_polygons_any_layer_inside_rect(
        &self,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PolygonId> {
        let envelope = rect.z_extruded_infinitely().aabb();

        self.polygons_rtree
            .as_ref()
            .locate_in_envelope(&envelope)
            .map(|geom_with_data| geom_with_data.data)
            .filter(move |&polygon_id| {
                let polygon = self.polygon(polygon_id);
                rect.contains_polygon(&polygon.spec.vertices)
            })
    }

    pub fn locate_nets_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_prefer_layer_intersecting_rect(rect) {
            if let Some(net) = self.joint(joint_id).spec.net {
                nets.insert(net);
            }
        }

        for segment_id in self.locate_segments_prefer_layer_intersecting_rect(rect) {
            if let Some(net) = self.segment(segment_id).net {
                nets.insert(net);
            }
        }

        for via_id in self.locate_vias_prefer_layer_intersecting_rect(rect) {
            if let Some(net) = self.via(via_id).net {
                nets.insert(net);
            }
        }

        for polygon_id in self.locate_polygons_prefer_layer_intersecting_rect(rect) {
            if let Some(net) = self.polygon(polygon_id).spec.net {
                nets.insert(net);
            }
        }

        nets.into_iter()
    }

    pub fn locate_nets_intersecting_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_intersecting_rect(rect) {
            if let Some(net) = self.joint(joint_id).spec.net {
                nets.insert(net);
            }
        }

        for segment_id in self.locate_segments_intersecting_rect(rect) {
            if let Some(net) = self.segment(segment_id).net {
                nets.insert(net);
            }
        }

        for via_id in self.locate_vias_intersecting_rect(rect) {
            if let Some(net) = self.via(via_id).net {
                nets.insert(net);
            }
        }

        for polygon_id in self.locate_polygons_intersecting_rect(rect) {
            if let Some(net) = self.polygon(polygon_id).spec.net {
                nets.insert(net);
            }
        }

        nets.into_iter()
    }

    pub fn locate_nets_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_prefer_layer_inside_rect(rect) {
            if let Some(net) = self.joint(joint_id).spec.net {
                nets.insert(net);
            }
        }

        for segment_id in self.locate_segments_prefer_layer_inside_rect(rect) {
            if let Some(net) = self.segment(segment_id).net {
                nets.insert(net);
            }
        }

        for via_id in self.locate_vias_prefer_layer_inside_rect(rect) {
            if let Some(net) = self.via(via_id).net {
                nets.insert(net);
            }
        }

        for polygon_id in self.locate_polygons_prefer_layer_inside_rect(rect) {
            if let Some(net) = self.polygon(polygon_id).spec.net {
                nets.insert(net);
            }
        }

        nets.into_iter()
    }

    pub fn locate_nets_inside_rect(&self, rect: Rect3<i64>) -> impl Iterator<Item = NetId> {
        let mut nets = BTreeSet::new();

        for joint_id in self.locate_joints_inside_rect(rect) {
            if let Some(net) = self.joint(joint_id).spec.net {
                nets.insert(net);
            }
        }

        for segment_id in self.locate_segments_inside_rect(rect) {
            if let Some(net) = self.segment(segment_id).net {
                nets.insert(net);
            }
        }

        for via_id in self.locate_vias_inside_rect(rect) {
            if let Some(net) = self.via(via_id).net {
                nets.insert(net);
            }
        }

        for polygon_id in self.locate_polygons_inside_rect(rect) {
            if let Some(net) = self.polygon(polygon_id).spec.net {
                nets.insert(net);
            }
        }

        nets.into_iter()
    }
}
