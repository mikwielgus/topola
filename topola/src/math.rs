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

macro_rules! impl_inside_polygon {
    ($type:ty) => {
        impl Vector2<$type> {
            // Checks if the point (px, py) is inside a polygon using the ray-casting
            // algorithm. Division is not used to avoid integer truncation errors.
            pub fn inside_polygon(&self, polygon: &[Vector2<$type>]) -> bool {
                let mut inside = false;
                let n = polygon.len();
                let px = self.x;
                let py = self.y;

                let mut v1 = &polygon[n - 1];

                for v2 in polygon.iter() {
                    let dy = v2.y - v1.y;
                    let zero = 0 as $type;

                    if dy != zero && (py > v1.y) != (py > v2.y) {
                        let dx = v2.x - v1.x;
                        let t = py - v1.y;
                        let s = px - v1.x;

                        let crosses = if dy > zero {
                            s * dy < dx * t
                        } else {
                            s * dy > dx * t
                        };

                        if crosses {
                            inside = !inside;
                        }
                    }
                    v1 = v2;
                }

                inside
            }
        }
    };
}

impl_inside_polygon!(f32);
impl_inside_polygon!(f64);
impl_inside_polygon!(i32);
impl_inside_polygon!(i64);

/// Returns the four vertices of a segment inflated by `half_width`, forming a convex
/// quadrilateral. The segment goes from (x1, y1) to (x2, y2).
pub fn inflated_segment(x1: i64, y1: i64, x2: i64, y2: i64, half_width: u64) -> [Vector2<i64>; 4] {
    let dx = x2 - x1;
    let dy = y2 - y1;

    let approx_len = std::cmp::max(dx.abs(), dy.abs()) + 3 * std::cmp::min(dx.abs(), dy.abs()) / 8;

    // Perpendicular vector scaled to half-width.
    let px = -dy * (half_width as i64) / approx_len;
    let py = dx * (half_width as i64) / approx_len;

    [
        Vector2::new(x1 + px, y1 + py),
        Vector2::new(x2 + px, y2 + py),
        Vector2::new(x2 - px, y2 - py),
        Vector2::new(x1 - px, y1 - py),
    ]
}

macro_rules! impl_rotate_around_point {
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

impl_rotate_around_point!(f32);
impl_rotate_around_point!(f64);

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

impl_polygon_centroid!(f32);
impl_polygon_centroid!(f64);
impl_polygon_centroid!(i32);
impl_polygon_centroid!(i64);
