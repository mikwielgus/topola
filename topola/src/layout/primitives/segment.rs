// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::vector::Vector2;
use crate::{Rect3, Vector3, layout::LayerId};

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
pub struct SegmentId(usize);

impl SegmentId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SegmentSpec {
    pub endjoints: [JointId; 2],
    pub half_width: u64,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub spec: SegmentSpec,
    pub endpoints: [Vector2<i64>; 2],
    pub layer: LayerId,
    pub net: Option<NetId>,
}

impl Segment {
    pub fn center(&self) -> Vector2<i64> {
        (self.endpoints[0] + self.endpoints[1]) / 2
    }

    pub fn contains_point2(&self, point: Vector2<i64>) -> bool {
        let vertices = crate::math::inflated_segment(
            self.endpoints[0].x,
            self.endpoints[0].y,
            self.endpoints[1].x,
            self.endpoints[1].y,
            self.spec.half_width,
        );
        point.inside_polygon(&vertices)
    }

    /// NOTE: This is not the bounding box. The output rectangle is in general
    /// not axis-aligned.
    pub fn bounding_rectangle(&self) -> [Vector2<i64>; 4] {
        crate::math::inflated_segment(
            self.endpoints[0].x,
            self.endpoints[0].y,
            self.endpoints[1].x,
            self.endpoints[1].y,
            self.spec.half_width,
        )
    }

    pub fn bbox(&self) -> Rect3<i64> {
        let endpoints = self.endpoints;
        let layer = self.layer.index() as i64;
        let half_width = self.spec.half_width as i64;

        Rect3::new(
            Vector3::new(
                std::cmp::min(endpoints[0].x, endpoints[1].x) - half_width,
                std::cmp::min(endpoints[0].y, endpoints[1].y) - half_width,
                layer,
            ),
            Vector3::new(
                std::cmp::max(endpoints[0].x, endpoints[1].x) + half_width,
                std::cmp::max(endpoints[0].y, endpoints[1].y) + half_width,
                layer,
            ),
        )
    }
}
