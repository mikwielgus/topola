// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{
    Add, AddAssign, Constructor, Div, DivAssign, From, Into, Mul, MulAssign, Sub, SubAssign,
};
use serde::{Deserialize, Serialize};

#[derive(
    Add,
    AddAssign,
    Clone,
    Constructor,
    Copy,
    Debug,
    Deserialize,
    Div,
    DivAssign,
    Eq,
    From,
    Into,
    Mul,
    MulAssign,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Sub,
    SubAssign,
)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

#[derive(
    Add,
    AddAssign,
    Clone,
    Constructor,
    Copy,
    Debug,
    Deserialize,
    Div,
    DivAssign,
    Eq,
    From,
    Into,
    Mul,
    MulAssign,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    Sub,
    SubAssign,
)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T: Copy> Vector3<T> {
    pub fn xy(self) -> Vector2<T> {
        Vector2::new(self.x, self.y)
    }
}

impl<T: Copy> From<[T; 2]> for Vector2<T> {
    fn from(from: [T; 2]) -> Self {
        Self {
            x: from[0],
            y: from[1],
        }
    }
}

impl<T: Copy> From<Vector2<T>> for [T; 2] {
    fn from(from: Vector2<T>) -> Self {
        [from.x, from.y]
    }
}

macro_rules! impl_vector2_inside_polygon {
    ($type:ty) => {
        impl Vector2<$type> {
            // Checks if the point is inside a polygon by casting a ray to the
            // right. Division is not used to avoid integer truncation errors.
            pub fn inside_polygon(&self, polygon: &[Vector2<$type>]) -> bool {
                let mut inside = false;
                let n = polygon.len();

                // `self` is `v0`.

                // `v1` is the previous vertex.
                let mut v1 = &polygon[n - 1];

                // `v2` is the current vertex.
                for v2 in polygon.iter() {
                    let dx12 = v2.x - v1.x;
                    let dy12 = v2.y - v1.y;

                    // First, check if the line of the horizontal rightward ray
                    // cast to actually crosses the vertical span of the current
                    // `(v1, v2)` edge.
                    if dy12 != (0 as $type) && (self.y > v1.y) != (self.y > v2.y) {
                        let dx01 = self.x - v1.x;
                        let dy01 = self.y - v1.y;

                        // Now check if the (v1, v2) edge is actually on the
                        // right side of the ray and not on the left.
                        //
                        // This just compares the X coordinate of `self` (`v0`)
                        // to the X coordinate of the intersection between the
                        // horizontal rightward ray and the current `(v1, v2)` edge:
                        //
                        // `self.x < v1.x + (self.y - v1.y) * (dx12 / dy12)`
                        //
                        // but is algebraically simplified and rewritten to not
                        // use division.
                        let crosses = if dy12 > (0 as $type) {
                            dx01 * dy12 < dx12 * dy01
                        } else {
                            dx01 * dy12 > dx12 * dy01
                        };

                        // Even-odd rule: flip whether the point is inside or
                        // outside upon each detected crossing.
                        if crosses {
                            inside = !inside;
                        }
                    }

                    // Make the current vertex previous for the next loop
                    // iteration.
                    v1 = v2;
                }

                inside
            }
        }
    };
}

impl_vector2_inside_polygon!(i8);
impl_vector2_inside_polygon!(i16);
impl_vector2_inside_polygon!(i32);
impl_vector2_inside_polygon!(i64);
impl_vector2_inside_polygon!(i128);
impl_vector2_inside_polygon!(u8);
impl_vector2_inside_polygon!(u16);
impl_vector2_inside_polygon!(u32);
impl_vector2_inside_polygon!(u64);
impl_vector2_inside_polygon!(u128);
impl_vector2_inside_polygon!(f32);
impl_vector2_inside_polygon!(f64);

macro_rules! impl_vector2_rotate_around_point {
    ($type:ty) => {
        impl Vector2<$type> {
            pub fn rotate_around_point(&mut self, angle: $type, origin: Vector2<$type>) -> Self {
                let sin = angle.sin();
                let cos = angle.cos();

                let tx = self.x - origin.x;
                let ty = self.y - origin.y;

                let rx = tx * cos - ty * sin;
                let ry = tx * sin + ty * cos;

                self.x = rx + origin.x;
                self.y = ry + origin.y;

                *self
            }

            pub fn rotate_around_point_degrees(
                &mut self,
                angle: $type,
                origin: Vector2<$type>,
            ) -> Self {
                self.rotate_around_point(angle.to_radians(), origin)
            }
        }
    };
}

impl_vector2_rotate_around_point!(f32);
impl_vector2_rotate_around_point!(f64);

macro_rules! impl_polygon_centroid {
    ($type:ty) => {
        impl Vector2<$type> {
            pub fn polygon_centroid(polygon: &[Vector2<$type>]) -> Self {
                let mut sum = Vector2::new(0 as $type, 0 as $type);

                for vertex in polygon.iter() {
                    sum += *vertex;
                }

                sum / polygon.len() as $type
            }
        }
    };
}

impl_polygon_centroid!(i8);
impl_polygon_centroid!(i16);
impl_polygon_centroid!(i32);
impl_polygon_centroid!(i64);
impl_polygon_centroid!(u8);
impl_polygon_centroid!(u16);
impl_polygon_centroid!(u32);
impl_polygon_centroid!(u64);
impl_polygon_centroid!(f32);
impl_polygon_centroid!(f64);
