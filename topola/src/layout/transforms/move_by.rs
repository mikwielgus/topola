// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    layout::{Layout, compounds::ComponentId},
    vector::Vector2,
};

impl Layout {
    pub fn move_component_by(&mut self, id: ComponentId, translation: Vector2<i64>) {
        self.move_components_by(&[id], translation);
    }

    pub fn move_components_by(&mut self, ids: &[ComponentId], translation: Vector2<i64>) {
        crate::profile_function!();
        for id in ids {
            let component = self.components[id.index()].clone();

            for &joint_id in &component.joints {
                self.modify_joint_raw(joint_id, |joint| joint.spec.position += translation);
            }

            for &seg_id in &component.segs {
                self.update_seg(seg_id);
            }

            for &via_id in &component.vias {
                self.update_via(via_id);
            }

            for &poly_id in &component.polys {
                self.modify_poly(poly_id, |poly| {
                    poly
                        .spec.vertices
                        .iter_mut()
                        .for_each(|vertex| *vertex += translation);
                    poly.centroid += translation;
                });
            }
        }
    }
}
