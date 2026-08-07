// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    layout::{
        Layout,
        compounds::{ComponentId, PinId},
        primitives::PrimitiveId,
    },
    vector::Vector2,
};

impl Layout {
    pub fn component_retentions(
        &self,
        violator: ComponentId,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.component(violator)
            .pins
            .iter()
            .flat_map(move |&pin_id| self.pin_retentions(pin_id))
    }

    pub fn pin_retentions(&self, violator: PinId) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.pin(violator)
            .primitives()
            .flat_map(move |primitive| self.primitive_retentions(primitive))
    }

    pub fn primitive_retentions(
        &self,
        violator: PrimitiveId,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        let boundary = self.place_boundary();
        self.primitive_bbox2(violator)
            .corners()
            .into_iter()
            .map(move |corner| Self::point_retention(corner, boundary))
    }

    fn point_retention(violator: Vector2<i64>, boundary: &[Vector2<i64>]) -> Vector2<i64> {
        if boundary.len() < 3 || violator.inside_polygon(boundary) {
            return Vector2::new(0, 0);
        }

        violator.closest_point_on_poly_boundary(boundary) - violator
    }
}
