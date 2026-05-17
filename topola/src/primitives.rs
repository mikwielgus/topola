// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use rstar::{AABB, Envelope, primitives::Rectangle};
use serde::{Deserialize, Serialize};

use crate::{
    Vector2,
    layout::{NetId, PinId},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PrimitiveId {
    Joint(JointId),
    Segment(SegmentId),
    Polygon(PolygonId),
}

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

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
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
    pub pin: Option<PinId>,
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub spec: SegmentSpec,
    pub endpoints: [Vector2<i64>; 2],
    pub layer: usize,
    pub net: NetId,
}

impl Segment {
    pub fn center(&self) -> Vector2<i64> {
        (self.endpoints[0] + self.endpoints[1]) / 2
    }

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        let vertices = crate::math::inflated_segment(
            self.endpoints[0].x,
            self.endpoints[0].y,
            self.endpoints[1].x,
            self.endpoints[1].y,
            self.spec.half_width,
        );
        point.inside_polygon(&vertices)
    }

    pub fn bbox(&self) -> [Vector2<i64>; 4] {
        crate::math::inflated_segment(
            self.endpoints[0].x,
            self.endpoints[0].y,
            self.endpoints[1].x,
            self.endpoints[1].y,
            self.spec.half_width,
        )
    }

    pub fn rtree_bbox(&self) -> Rectangle<[i64; 3]> {
        let endpoints = self.endpoints;
        let layer = self.layer as i64;
        let half_width = self.spec.half_width as i64;

        let min_x = std::cmp::min(endpoints[0].x, endpoints[1].x) - half_width;
        let min_y = std::cmp::min(endpoints[0].y, endpoints[1].y) - half_width;
        let max_x = std::cmp::max(endpoints[0].x, endpoints[1].x) + half_width;
        let max_y = std::cmp::max(endpoints[0].y, endpoints[1].y) + half_width;

        Rectangle::from_corners([min_x, min_y, layer], [max_x, max_y, layer])
    }
}

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
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
pub struct Via {
    pub endjoints: [JointId; 2],
    pub layer: usize, // ??? This should be a range.
    pub radius: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

impl Via {
    /*pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        //
    }*/
}

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
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
    pub layer: usize,
    pub net: NetId,
    pub pin: Option<PinId>,
}

impl Polygon {
    pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        Rectangle::from_aabb(
            self.vertices
                .clone()
                .into_iter()
                .fold(AABB::new_empty(), |aabb, vertex| {
                    aabb.merged(&AABB::from_point([vertex.x, vertex.y, self.layer as i64]))
                }),
        )
    }

    pub fn center(&self) -> Vector2<i64> {
        Vector2::<i64>::polygon_centroid(&self.vertices)
    }

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        point.inside_polygon(&self.vertices)
    }
}
