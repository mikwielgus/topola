// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2,
    layout::{
        Layout,
        primitives::{JointId, PolyId, SegId, ViaId},
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

    pub fn joint_seg_rect_overlap(
        &self,
        infringer: JointId,
        infringee: SegId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.seg(infringee).bbox().xy();

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

    pub fn joint_poly_rect_overlap(
        &self,
        infringer: JointId,
        infringee: PolyId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.joint(infringer).bbox().xy();
        let infringee_bbox = self.poly(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn seg_joint_rect_overlap(
        &self,
        infringer: SegId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.seg(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn seg_seg_rect_overlap(
        &self,
        infringer: SegId,
        infringee: SegId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.seg(infringer).bbox().xy();
        let infringee_bbox = self.seg(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn seg_via_rect_overlap(
        &self,
        infringer: SegId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.seg(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn seg_poly_rect_overlap(
        &self,
        infringer: SegId,
        infringee: PolyId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.seg(infringer).bbox().xy();
        let infringee_bbox = self.poly(infringee).bbox().xy();

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

    pub fn via_seg_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: SegId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.seg(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_via_rect_overlap(&self, infringer: ViaId, infringee: ViaId) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn via_poly_rect_overlap(
        &self,
        infringer: ViaId,
        infringee: PolyId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.via(infringer).bbox().xy();
        let infringee_bbox = self.poly(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn poly_joint_rect_overlap(
        &self,
        infringer: PolyId,
        infringee: JointId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.poly(infringer).bbox().xy();
        let infringee_bbox = self.joint(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn poly_seg_rect_overlap(
        &self,
        infringer: PolyId,
        infringee: SegId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.poly(infringer).bbox().xy();
        let infringee_bbox = self.seg(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn poly_via_rect_overlap(
        &self,
        infringer: PolyId,
        infringee: ViaId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.poly(infringer).bbox().xy();
        let infringee_bbox = self.via(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }

    pub fn poly_poly_rect_overlap(
        &self,
        infringer: PolyId,
        infringee: PolyId,
    ) -> Option<Rect2<i64>> {
        let infringer_bbox = self.poly(infringer).bbox().xy();
        let infringee_bbox = self.poly(infringee).bbox().xy();

        infringer_bbox.intersection(infringee_bbox)
    }
}
