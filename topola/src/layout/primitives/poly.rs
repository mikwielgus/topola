// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};

use crate::layout::compounds::{ComponentId, NetId, PinId};
use crate::vector::Vector2;
use crate::{Rect3, Vector3, layout::LayerId};

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
pub struct PolyId(usize);

impl PolyId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct PolySpec {
    pub vertices: Vec<Vector2<i64>>,
    pub layer: LayerId,
    pub net: Option<NetId>,
    pub component: Option<ComponentId>,
    pub pin: Option<PinId>,
}

#[derive(Clone, Debug)]
pub struct Poly {
    pub spec: PolySpec,
    pub centroid: Vector2<i64>,
}

impl Poly {
    pub fn new(spec: PolySpec) -> Self {
        Self {
            centroid: Vector2::<i64>::poly_centroid(&spec.vertices),
            spec,
        }
    }

    pub fn bbox(&self) -> Rect3<i64> {
        let layer = self.spec.layer.index() as i64;
        let mut min = Vector2::new(i64::MAX, i64::MAX);
        let mut max = Vector2::new(i64::MIN, i64::MIN);

        for vertex in &self.spec.vertices {
            min.x = std::cmp::min(min.x, vertex.x);
            min.y = std::cmp::min(min.y, vertex.y);
            max.x = std::cmp::max(max.x, vertex.x);
            max.y = std::cmp::max(max.y, vertex.y);
        }

        if self.spec.vertices.is_empty() {
            return Rect3::new(Vector3::new(0, 0, layer), Vector3::new(0, 0, layer));
        }

        Rect3::new(
            Vector3::new(min.x, min.y, layer),
            Vector3::new(max.x, max.y, layer),
        )
    }

    pub fn contains_point2(&self, point: Vector2<i64>) -> bool {
        point.inside_polygon(&self.spec.vertices)
    }
}
