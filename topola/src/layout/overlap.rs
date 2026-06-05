// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2,
    layout::{
        Layout,
        primitives::{JointId, PolygonId, SegmentId, ViaId},
    },
};

impl Layout {
    pub fn joint_joint_rect_overlap(
        &self,
        infringer: JointId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_segment_rect_overlap(
        &self,
        infringer: JointId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_via_rect_overlap(
        &self,
        infringer: JointId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_polygon_rect_overlap(
        &self,
        infringer: JointId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_joint_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.segment(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_segment_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.segment(infringer).bbox().xy();
        let infringee_bbox = self.segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_via_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.segment(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_polygon_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.segment(infringer).bbox().xy();
        let infringee_bbox = self.polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_joint_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_segment_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_via_rect_overlap(&self, infringer: ViaId, infringee: ViaId) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_polygon_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_joint_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.polygon(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_segment_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.polygon(infringer).bbox().xy();
        let infringee_bbox = self.segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_via_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.polygon(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_polygon_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.polygon(infringer).bbox().xy();
        let infringee_bbox = self.polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }
}
