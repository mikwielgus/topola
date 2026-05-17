// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use rstar::{AABB, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::layout::{NetId, PinId};
use crate::math::Vector2;

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct JointId(usize);

impl JointId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Joint {
    pub position: Vector2<i64>,
    pub layer: usize,
    pub radius: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

impl Joint {
    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        Rectangle::from_aabb(AABB::from_corners(
            [
                self.position.x - self.radius as i64,
                self.position.y - self.radius as i64,
                self.layer as i64,
            ],
            [
                self.position.x + self.radius as i64,
                self.position.y + self.radius as i64,
                self.layer as i64,
            ],
        ))
    }

    pub fn center(&self) -> Vector2<i64> {
        self.position
    }

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        (point.x - self.position.x).pow(2) as u64 + (point.y - self.position.y).pow(2) as u64
            <= self.radius.pow(2)
    }
}
