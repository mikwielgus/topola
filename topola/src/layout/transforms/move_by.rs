// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{Layout, layout::compounds::ComponentId, vector::Vector2};

impl Layout {
    pub fn move_component_by(&mut self, id: ComponentId, translation: Vector2<i64>) {
        self.move_components_by(&[id], translation);
    }

    pub fn move_components_by(&mut self, ids: &[ComponentId], translation: Vector2<i64>) {
        for id in ids {
            let component = self.components[id.index()].clone();

            for &joint_id in &component.joints {
                self.modify_joint_raw(joint_id, |joint| joint.spec.position += translation);
            }

            for &segment_id in &component.segments {
                self.update_segment(segment_id);
            }

            for &via_id in &component.vias {
                self.update_via(via_id);
            }

            for &polygon_id in &component.polygons {
                self.modify_polygon(polygon_id, |polygon| {
                    polygon
                        .vertices
                        .iter_mut()
                        .for_each(|vertex| *vertex += translation)
                });
            }
        }
    }
}
