// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use rstar::{AABB, Envelope, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::compounds::{ComponentId, NetId, PinId};
use crate::layout::LayerId;
use crate::math::Vector2;

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
pub struct PolygonId(usize);

impl PolygonId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Polygon {
    pub vertices: Vec<Vector2<i64>>,
    pub layer: LayerId,
    pub net: NetId,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

impl Polygon {
    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        Rectangle::from_aabb(self.vertices.clone().into_iter().fold(
            AABB::new_empty(),
            |aabb, vertex| {
                aabb.merged(&AABB::from_point([
                    vertex.x,
                    vertex.y,
                    self.layer.index() as i64,
                ]))
            },
        ))
    }

    pub fn center(&self) -> Vector2<i64> {
        Vector2::<i64>::polygon_centroid(&self.vertices)
    }

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        point.inside_polygon(&self.vertices)
    }
}
