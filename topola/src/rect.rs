// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use derive_more::Constructor;
use num_traits::Bounded;
use rstar::{AABB, RTreeNum};
use serde::{Deserialize, Serialize};

use crate::{Vector2, Vector3};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Getters, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct Rect2<T> {
    pub min: Vector2<T>,
    pub max: Vector2<T>,
}

impl<T: Copy + Ord> Rect2<T> {
    pub fn z_extruded(self, from: T, to: T) -> Rect3<T> {
        Rect3 {
            min: Vector3::new(self.min.x, self.min.y, std::cmp::min(from, to)),
            max: Vector3::new(self.max.x, self.max.y, std::cmp::max(from, to)),
        }
    }
}

impl<T: Bounded + Copy> Rect2<T> {
    pub fn z_extruded_infinitely(self) -> Rect3<T> {
        Rect3 {
            min: Vector3::new(self.min.x, self.min.y, Bounded::min_value()),
            max: Vector3::new(self.max.x, self.max.y, Bounded::max_value()),
        }
    }
}

impl<T: Copy> Rect2<T> {
    pub fn corners(&self) -> [Vector2<T>; 4] {
        [
            self.min,
            Vector2::new(self.max.x, self.min.y),
            self.max,
            Vector2::new(self.min.x, self.max.y),
        ]
    }
}

impl<T: RTreeNum> Rect2<T> {
    pub fn aabb(self) -> AABB<[T; 2]> {
        AABB::from_corners([self.min.x, self.min.y], [self.max.x, self.max.y])
    }
}

macro_rules! impl_rect2_contains_point {
    ($type:ty) => {
        impl Rect2<$type> {
            pub fn contains_point(&self, point: Vector2<$type>) -> bool {
                point.x >= self.min.x
                    && point.x <= self.max.x
                    && point.y >= self.min.y
                    && point.y <= self.max.y
            }
        }
    };
}

impl_rect2_contains_point!(i8);
impl_rect2_contains_point!(i16);
impl_rect2_contains_point!(i32);
impl_rect2_contains_point!(i64);
impl_rect2_contains_point!(i128);
impl_rect2_contains_point!(u8);
impl_rect2_contains_point!(u16);
impl_rect2_contains_point!(u32);
impl_rect2_contains_point!(u64);
impl_rect2_contains_point!(u128);
impl_rect2_contains_point!(f32);
impl_rect2_contains_point!(f64);

macro_rules! impl_rect2_contains_circle {
    ($type:ty) => {
        impl Rect2<$type> {
            pub fn contains_circle(&self, center: Vector2<$type>, radius: $type) -> bool {
                let min_x = self.min.x as f64;
                let min_y = self.min.y as f64;
                let max_x = self.max.x as f64;
                let max_y = self.max.y as f64;
                let cx = center.x as f64;
                let cy = center.y as f64;
                let r = radius as f64;

                cx - r >= min_x && cx + r <= max_x && cy - r >= min_y && cy + r <= max_y
            }
        }
    };
}

impl_rect2_contains_circle!(i8);
impl_rect2_contains_circle!(i16);
impl_rect2_contains_circle!(i32);
impl_rect2_contains_circle!(i64);
impl_rect2_contains_circle!(i128);
impl_rect2_contains_circle!(u8);
impl_rect2_contains_circle!(u16);
impl_rect2_contains_circle!(u32);
impl_rect2_contains_circle!(u64);
impl_rect2_contains_circle!(u128);
impl_rect2_contains_circle!(f32);
impl_rect2_contains_circle!(f64);

macro_rules! impl_rect2_intersects_circle {
    ($type:ty) => {
        impl Rect2<$type> {
            pub fn intersects_circle(&self, center: Vector2<$type>, radius: $type) -> bool {
                let min_x = self.min.x as f64;
                let min_y = self.min.y as f64;
                let max_x = self.max.x as f64;
                let max_y = self.max.y as f64;
                let cx = center.x as f64;
                let cy = center.y as f64;
                let r = radius as f64;

                let closest_x = cx.clamp(min_x, max_x);
                let closest_y = cy.clamp(min_y, max_y);
                let dx = cx - closest_x;
                let dy = cy - closest_y;

                dx * dx + dy * dy <= r * r
            }
        }
    };
}

impl_rect2_intersects_circle!(i8);
impl_rect2_intersects_circle!(i16);
impl_rect2_intersects_circle!(i32);
impl_rect2_intersects_circle!(i64);
impl_rect2_intersects_circle!(i128);
impl_rect2_intersects_circle!(u8);
impl_rect2_intersects_circle!(u16);
impl_rect2_intersects_circle!(u32);
impl_rect2_intersects_circle!(u64);
impl_rect2_intersects_circle!(u128);
impl_rect2_intersects_circle!(f32);
impl_rect2_intersects_circle!(f64);

macro_rules! impl_rect2_contains_polygon {
    ($type:ty) => {
        impl Rect2<$type> {
            pub fn contains_polygon(&self, polygon: &[Vector2<$type>]) -> bool {
                let corners = self.corners();
                polygon.iter().all(|point| point.inside_polygon(&corners))
            }
        }
    };
}

impl_rect2_contains_polygon!(i8);
impl_rect2_contains_polygon!(i16);
impl_rect2_contains_polygon!(i32);
impl_rect2_contains_polygon!(i64);
impl_rect2_contains_polygon!(i128);
impl_rect2_contains_polygon!(u8);
impl_rect2_contains_polygon!(u16);
impl_rect2_contains_polygon!(u32);
impl_rect2_contains_polygon!(u64);
impl_rect2_contains_polygon!(u128);
impl_rect2_contains_polygon!(f32);
impl_rect2_contains_polygon!(f64);

macro_rules! impl_rect2_intersects_polygon {
    ($type:ty) => {
        impl Rect2<$type> {
            pub fn intersects_polygon(&self, polygon: &[Vector2<$type>]) -> bool {
                if polygon.is_empty() {
                    return false;
                }

                if polygon.iter().any(|&vertex| self.contains_point(vertex)) {
                    return true;
                }

                if self
                    .corners()
                    .iter()
                    .any(|corner| corner.inside_polygon(polygon))
                {
                    return true;
                }

                false

                // TODO: Now handle cases where no vertex is inside but only
                // edges intersect.
            }
        }
    };
}

impl_rect2_intersects_polygon!(i8);
impl_rect2_intersects_polygon!(i16);
impl_rect2_intersects_polygon!(i32);
impl_rect2_intersects_polygon!(i64);
impl_rect2_intersects_polygon!(i128);
impl_rect2_intersects_polygon!(u8);
impl_rect2_intersects_polygon!(u16);
impl_rect2_intersects_polygon!(u32);
impl_rect2_intersects_polygon!(u64);
impl_rect2_intersects_polygon!(u128);
impl_rect2_intersects_polygon!(f32);
impl_rect2_intersects_polygon!(f64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Getters, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Rect3<T> {
    pub min: Vector3<T>,
    pub max: Vector3<T>,
}

impl<T: Ord + Copy> Rect3<T> {
    pub fn new(from: Vector3<T>, to: Vector3<T>) -> Self {
        Self {
            min: Vector3::new(
                std::cmp::min(from.x, to.x),
                std::cmp::min(from.y, to.y),
                std::cmp::min(from.z, to.z),
            ),
            max: Vector3::new(
                std::cmp::max(from.x, to.x),
                std::cmp::max(from.y, to.y),
                std::cmp::max(from.z, to.z),
            ),
        }
    }
}

impl<T: Copy + Ord> Rect3<T> {
    pub fn xy(self) -> Rect2<T> {
        Rect2::new(self.min.xy(), self.max.xy())
    }
}

impl<T: RTreeNum> Rect3<T> {
    pub fn aabb(self) -> AABB<[T; 3]> {
        AABB::from_corners(
            [self.min.x, self.min.y, self.min.z],
            [self.max.x, self.max.y, self.max.z],
        )
    }
}
