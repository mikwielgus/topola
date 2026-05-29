// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2,
    layout::{Layout, compounds::ComponentId},
};

impl Layout {
    pub fn component_bbox2(&self, component_id: ComponentId) -> Option<Rect2<i64>> {
        let component = self.component(component_id);
        let mut maybe_accum_bbox2 = None::<Rect2<i64>>;

        for &joint_id in &component.joints {
            unionize(&mut maybe_accum_bbox2, self.joint(joint_id).bbox().xy());
        }

        for &segment_id in &component.segments {
            unionize(&mut maybe_accum_bbox2, self.segment(segment_id).bbox().xy());
        }

        for &via_id in &component.vias {
            unionize(&mut maybe_accum_bbox2, self.via(via_id).bbox().xy());
        }

        for &polygon_id in &component.polygons {
            unionize(&mut maybe_accum_bbox2, self.polygon(polygon_id).bbox().xy());
        }

        maybe_accum_bbox2
    }

    pub fn components_bbox2(
        &self,
        component_ids: impl IntoIterator<Item = ComponentId>,
    ) -> Option<Rect2<i64>> {
        let mut maybe_accum_bbox2 = None::<Rect2<i64>>;

        for component_id in component_ids {
            if let Some(component_bbox2) = self.component_bbox2(component_id) {
                unionize(&mut maybe_accum_bbox2, component_bbox2);
            }
        }

        maybe_accum_bbox2
    }
}

fn unionize(maybe_accum_bbox2: &mut Option<Rect2<i64>>, bbox2_to_add: Rect2<i64>) {
    *maybe_accum_bbox2 = Some(match maybe_accum_bbox2 {
        None => bbox2_to_add,
        Some(accum) => accum.union(bbox2_to_add),
    })
}
