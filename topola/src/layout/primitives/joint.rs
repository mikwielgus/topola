// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use rstar::{AABB, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::compounds::{ComponentId, NetId, PinId};
use crate::math::Vector2;

use super::{SegmentId, ViaId};

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
pub struct JointSpec {
    pub position: Vector2<i64>,
    pub layer: usize,
    pub radius: u64,
    pub net: NetId,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

#[derive(Clone, Debug)]
pub struct Joint {
    pub spec: JointSpec,
    pub segments: Vec<SegmentId>,
    pub vias: Vec<ViaId>,
}

impl Joint {
    pub fn center(&self) -> Vector2<i64> {
        self.spec.position
    }

    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        Rectangle::from_aabb(AABB::from_corners(
            [
                self.spec.position.x - self.spec.radius as i64,
                self.spec.position.y - self.spec.radius as i64,
                self.spec.layer as i64,
            ],
            [
                self.spec.position.x + self.spec.radius as i64,
                self.spec.position.y + self.spec.radius as i64,
                self.spec.layer as i64,
            ],
        ))
    }

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        (point.x - self.spec.position.x).pow(2) as u64
            + (point.y - self.spec.position.y).pow(2) as u64
            <= self.spec.radius.pow(2)
    }
}
