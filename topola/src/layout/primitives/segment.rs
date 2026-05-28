// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use rstar::primitives::Rectangle;
use serde::{Deserialize, Serialize};

use crate::layout::LayerId;
use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::vector::Vector2;

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
    pub net: NetId,
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

    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        let endpoints = self.endpoints;
        let layer = self.layer.index() as i64;
        let half_width = self.spec.half_width as i64;

        let min_x = std::cmp::min(endpoints[0].x, endpoints[1].x) - half_width;
        let min_y = std::cmp::min(endpoints[0].y, endpoints[1].y) - half_width;
        let max_x = std::cmp::max(endpoints[0].x, endpoints[1].x) + half_width;
        let max_y = std::cmp::max(endpoints[0].y, endpoints[1].y) + half_width;

        Rectangle::from_corners([min_x, min_y, layer], [max_x, max_y, layer])
    }
}
