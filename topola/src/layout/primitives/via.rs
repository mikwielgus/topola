// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::vector::Vector2;
use crate::{Rect3, Vector3, layout::LayerId};

use super::{Joint, JointId};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViaSpec {
    pub endjoints: [JointId; 2],
    pub radius: u64,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Via {
    pub spec: ViaSpec,
    pub position: Vector2<i64>,
    pub min_layer: LayerId,
    pub max_layer: LayerId,
    pub net: Option<NetId>,
}

impl Via {
    pub fn new(spec: ViaSpec, endjoints: [&Joint; 2]) -> Self {
        Self {
            spec,
            position: endjoints[0].spec.position,
            min_layer: std::cmp::min(endjoints[0].spec.layer, endjoints[1].spec.layer),
            max_layer: std::cmp::max(endjoints[0].spec.layer, endjoints[1].spec.layer),
            net: endjoints[0].spec.net,
        }
    }

    pub fn contains_point2(&self, point: Vector2<i64>) -> bool {
        (point.x - self.position.x).pow(2) as u64 + (point.y - self.position.y).pow(2) as u64
            <= self.spec.radius.pow(2)
    }

    pub fn bbox(&self) -> Rect3<i64> {
        let radius = self.spec.radius as i64;

        Rect3::new(
            Vector3::new(
                self.position.x - radius,
                self.position.y - radius,
                self.min_layer.index() as i64,
            ),
            Vector3::new(
                self.position.x + radius,
                self.position.y + radius,
                self.max_layer.index() as i64,
            ),
        )
    }
}
