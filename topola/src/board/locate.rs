// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Vector2,
    board::Board,
    layout::LayerId,
    selections::{ComponentSelector, PinSelector},
};

impl Board {
    pub fn locate_component_at_point(
        &self,
        layer: LayerId,
        point: Vector2<i64>,
    ) -> Option<ComponentSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_component_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_component_selector(segment_id);
        }

        // TODO: Vias.

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_component_selector(polygon_id);
        }

        None
    }

    pub fn locate_pin_at_point(&self, layer: LayerId, point: Vector2<i64>) -> Option<PinSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_pin_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_pin_selector(segment_id);
        }

        // TODO: Vias.

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_pin_selector(polygon_id);
        }

        None
    }
}
