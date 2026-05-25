// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use rstar::{AABB, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::layout::LayerId;
use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::math::Vector2;

use super::JointId;

#[derive(
    Clone,
    Constructor,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    From,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct ViaId(usize);

impl ViaId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ViaSpec {
    pub endjoints: [JointId; 2],
    pub radius: u64,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug)]
pub struct Via {
    pub spec: ViaSpec,
    pub position: Vector2<i64>,
    pub min_layer: LayerId,
    pub max_layer: LayerId,
    pub net: NetId,
}

impl Via {
    pub fn contains_point(&self, layer: LayerId, point: Vector2<i64>) -> bool {
        if layer < self.min_layer || layer > self.max_layer {
            return false;
        }

        (point.x - self.position.x).pow(2) as u64 + (point.y - self.position.y).pow(2) as u64
            <= self.spec.radius.pow(2)
    }

    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        let radius = self.spec.radius as i64;

        Rectangle::from_aabb(AABB::from_corners(
            [
                self.position.x - radius,
                self.position.y - radius,
                self.min_layer.index() as i64,
            ],
            [
                self.position.x + radius,
                self.position.y + radius,
                self.max_layer.index() as i64,
            ],
        ))
    }
}
