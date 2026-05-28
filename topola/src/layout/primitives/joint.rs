// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use rstar::{AABB, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::layout::LayerId;
use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::primitives::{SegmentId, ViaId};
use crate::vector::Vector2;

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
    pub layer: LayerId,
    pub radius: u64,
    pub net: Option<NetId>,
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
                self.spec.layer.index() as i64,
            ],
            [
                self.spec.position.x + self.spec.radius as i64,
                self.spec.position.y + self.spec.radius as i64,
                self.spec.layer.index() as i64,
            ],
        ))
    }

    pub fn contains_point2(&self, point: Vector2<i64>) -> bool {
        (point.x - self.spec.position.x).pow(2) as u64
            + (point.y - self.spec.position.y).pow(2) as u64
            <= self.spec.radius.pow(2)
    }
}
