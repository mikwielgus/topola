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

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct JointId(usize);

impl JointId {
    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
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
    pub fn id(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub endjoints: [JointId; 2],
    pub layer: usize,
    pub half_width: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct ViaId(usize);

impl ViaId {
    /// Returns the underlying index.
    #[inline]
    pub fn id(self) -> usize {
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
    pub fn id(self) -> usize {
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

    pub fn contains_point(&self, point: Vector2<i64>) -> bool {
        point.inside_polygon(&self.vertices)
    }
}
