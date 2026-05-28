// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use crate::{
    Rect3, Vector3,
    board::Board,
    selections::{ComponentSelector, NetSelector, PinSelector},
};

impl Board {
    pub fn locate_components_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_prefer_layer_at_point(point)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_prefer_layer_at_point(point)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_at_point(point)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_prefer_layer_at_point(point)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_components_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_at_point(point)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_at_point(point)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_at_point(point)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_at_point(point)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_components_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_prefer_layer_intersecting_rect(rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_prefer_layer_intersecting_rect(rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_intersecting_rect(rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_prefer_layer_intersecting_rect(rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_components_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_intersecting_rect(rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_intersecting_rect(rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_intersecting_rect(rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_intersecting_rect(rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_components_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_prefer_layer_inside_rect(rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_prefer_layer_inside_rect(rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_inside_rect(rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_prefer_layer_inside_rect(rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_components_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = ComponentSelector> + '_ {
        self.layout
            .locate_joints_prefer_layer_inside_rect(rect)
            .filter_map(|joint_id| self.joint_component_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_prefer_layer_inside_rect(rect)
                    .filter_map(|segment_id| self.segment_component_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_inside_rect(rect)
                    .filter_map(|via_id| self.via_component_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_prefer_layer_inside_rect(rect)
                    .filter_map(|polygon_id| self.polygon_component_selector(polygon_id)),
            )
    }

    pub fn locate_nets_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        self.layout
            .locate_joints_prefer_layer_at_point(point)
            .filter_map(|joint_id| self.joint_net_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_prefer_layer_at_point(point)
                    .filter_map(|segment_id| self.segment_net_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_at_point(point)
                    .filter_map(|via_id| self.via_net_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_prefer_layer_at_point(point)
                    .filter_map(|polygon_id| self.polygon_net_selector(polygon_id)),
            )
    }

    pub fn locate_nets_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        self.layout
            .locate_joints_at_point(point)
            .filter_map(|joint_id| self.joint_net_selector(joint_id))
            .chain(
                self.layout
                    .locate_segments_at_point(point)
                    .filter_map(|segment_id| self.segment_net_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_at_point(point)
                    .filter_map(|via_id| self.via_net_selector(via_id)),
            )
            .chain(
                self.layout
                    .locate_polygons_at_point(point)
                    .filter_map(|polygon_id| self.polygon_net_selector(polygon_id)),
            )
    }

    pub fn locate_nets_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        let mut selectors = BTreeSet::new();

        for net_id in self.layout.locate_nets_prefer_layer_intersecting_rect(rect) {
            let Some(net_name) = self.net_name(net_id) else {
                continue;
            };

            selectors.insert(NetSelector::new(net_name.to_string()));
        }

        selectors.into_iter()
    }

    pub fn locate_nets_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        let mut selectors = BTreeSet::new();

        for net_id in self.layout.locate_nets_intersecting_rect(rect) {
            let Some(net_name) = self.net_name(net_id) else {
                continue;
            };

            selectors.insert(NetSelector::new(net_name.to_string()));
        }

        selectors.into_iter()
    }

    pub fn locate_nets_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        let mut selectors = BTreeSet::new();

        for net_id in self.layout.locate_nets_prefer_layer_inside_rect(rect) {
            let Some(net_name) = self.net_name(net_id) else {
                continue;
            };

            selectors.insert(NetSelector::new(net_name.to_string()));
        }

        selectors.into_iter()
    }

    pub fn locate_nets_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = NetSelector> + '_ {
        let mut selectors = BTreeSet::new();

        for net_id in self.layout.locate_nets_inside_rect(rect) {
            let Some(net_name) = self.net_name(net_id) else {
                continue;
            };

            selectors.insert(NetSelector::new(net_name.to_string()));
        }

        selectors.into_iter()
    }

    pub fn locate_pins_prefer_layer_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_prefer_layer_at_point(point)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_prefer_layer_at_point(point)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_prefer_layer_at_point(point)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_at_point(point)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }

    pub fn locate_pins_at_point(
        &self,
        point: Vector3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_at_point(point)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_at_point(point)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_at_point(point)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_at_point(point)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }

    pub fn locate_pins_prefer_layer_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_prefer_layer_intersecting_rect(rect)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_prefer_layer_intersecting_rect(rect)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_prefer_layer_intersecting_rect(rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_intersecting_rect(rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }

    pub fn locate_pins_intersecting_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_intersecting_rect(rect)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_intersecting_rect(rect)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_intersecting_rect(rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_intersecting_rect(rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }

    pub fn locate_pins_prefer_layer_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_prefer_layer_inside_rect(rect)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_prefer_layer_inside_rect(rect)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_prefer_layer_inside_rect(rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_prefer_layer_inside_rect(rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }

    pub fn locate_pins_inside_rect(
        &self,
        rect: Rect3<i64>,
    ) -> impl Iterator<Item = PinSelector> + '_ {
        self.layout
            .locate_polygons_inside_rect(rect)
            .filter_map(|polygon_id| self.polygon_pin_selector(polygon_id))
            .chain(
                self.layout
                    .locate_joints_inside_rect(rect)
                    .filter_map(|joint_id| self.joint_pin_selector(joint_id)),
            )
            .chain(
                self.layout
                    .locate_segments_inside_rect(rect)
                    .filter_map(|segment_id| self.segment_pin_selector(segment_id)),
            )
            .chain(
                self.layout
                    .locate_vias_inside_rect(rect)
                    .filter_map(|via_id| self.via_pin_selector(via_id)),
            )
    }
}
