// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2,
    board::Board,
    primitives::{JointId, PolygonId, SegmentId, ViaId},
};

impl Board {
    pub fn joint_joint_rect_overlap(
        &self,
        infringer: JointId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().joint(infringer).bbox().xy();
        let infringee_bbox = self.layout().joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_segment_rect_overlap(
        &self,
        infringer: JointId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().joint(infringer).bbox().xy();
        let infringee_bbox = self.layout().segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_via_rect_overlap(
        &self,
        infringer: JointId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().joint(infringer).bbox().xy();
        let infringee_bbox = self.layout().via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn joint_polygon_rect_overlap(
        &self,
        infringer: JointId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().joint(infringer).bbox().xy();
        let infringee_bbox = self.layout().polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_joint_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().segment(infringer).bbox().xy();
        let infringee_bbox = self.layout().joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_segment_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().segment(infringer).bbox().xy();
        let infringee_bbox = self.layout().segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_via_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().segment(infringer).bbox().xy();
        let infringee_bbox = self.layout().via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn segment_polygon_rect_overlap(
        &self,
        infringer: SegmentId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().segment(infringer).bbox().xy();
        let infringee_bbox = self.layout().polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_joint_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().via(infringer).bbox().xy();
        let infringee_bbox = self.layout().joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_segment_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().via(infringer).bbox().xy();
        let infringee_bbox = self.layout().segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_via_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().via(infringer).bbox().xy();
        let infringee_bbox = self.layout().via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_polygon_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().via(infringer).bbox().xy();
        let infringee_bbox = self.layout().polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_joint_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().polygon(infringer).bbox().xy();
        let infringee_bbox = self.layout().joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_segment_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: SegmentId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().polygon(infringer).bbox().xy();
        let infringee_bbox = self.layout().segment(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_via_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().polygon(infringer).bbox().xy();
        let infringee_bbox = self.layout().via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn polygon_polygon_rect_overlap(
        &self,
        infringer: PolygonId,
        infringee: PolygonId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.layout().polygon(infringer).bbox().xy();
        let infringee_bbox = self.layout().polygon(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }
}
