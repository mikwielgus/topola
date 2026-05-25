// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Rect2, Vector2,
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

        if let Some(via_id) = self.layout.locate_vias_at_point(layer, point).next() {
            return self.via_component_selector(via_id);
        }

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_component_selector(polygon_id);
        }

        None
    }

    pub fn locate_component_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_intersecting_rect(layer, rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_intersecting_rect(layer, rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_intersecting_rect(layer, rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_intersecting_rect(layer, rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_component_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_inside_rect(layer, rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_inside_rect(layer, rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_inside_rect(layer, rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_inside_rect(layer, rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_pin_at_point(&self, layer: LayerId, point: Vector2<i64>) -> Option<PinSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_pin_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_pin_selector(segment_id);
        }

        if let Some(via_id) = self.layout.locate_vias_at_point(layer, point).next() {
            return self.via_pin_selector(via_id);
        }

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_pin_selector(polygon_id);
        }

        None
    }

    pub fn locate_pin_intersecting_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_joints_intersecting_rect(layer, rect)
            .filter_map(|joint_id| self.joint_pin_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_intersecting_rect(layer, rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_intersecting_rect(layer, rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_intersecting_rect(layer, rect)
                    .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id)),
            )
    }

    pub fn locate_pin_inside_rect(
        &self,
        layer: LayerId,
        rect: Rect2<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_joints_inside_rect(layer, rect)
            .filter_map(|joint_id| self.joint_pin_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_inside_rect(layer, rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_inside_rect(layer, rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_inside_rect(layer, rect)
                    .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id)),
            )
    }
}
