// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    layout::{
        Layout,
        compounds::{ComponentId, PinId},
    },
    vector::Vector2,
};

impl Layout {
    pub fn component_attractions(
        &self,
        attractee: ComponentId,
    ) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.component(attractee)
            .pins
            .iter()
            .flat_map(move |&pin_id| self.pin_attractions(pin_id))
    }

    pub fn pin_attractions(&self, attractee: PinId) -> impl Iterator<Item = Vector2<i64>> + '_ {
        self.pin(attractee)
            .spec
            .net
            .into_iter()
            .flat_map(move |net_id| {
                self.nets[net_id.index()]
                    .pins
                    .iter()
                    .copied()
                    .filter(move |&attractor| attractor != attractee)
                    .map(move |attractor| self.pin_pin_attraction(attractee, attractor))
            })
    }

    pub fn pin_pin_attraction(&self, attractee: PinId, attractor: PinId) -> Vector2<i64> {
        self.pin_centroid(attractor) - self.pin_centroid(attractee)
    }
}
