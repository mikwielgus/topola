// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Add, Constructor, From, Into, Sub};

#[derive(Add, Clone, Constructor, Copy, Debug, Eq, From, Into, PartialEq, Sub)]
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

// Check if the point (px, py) is inside the polygon using the ray-casting
// algorithm.
macro_rules! impl_inside_polygon {
    ($type:ty) => {
        impl Vector2<$type> {
            pub fn inside_polygon(&self, polygon: &[Vector2<$type>]) -> bool {
                let mut inside = false;
                let n = polygon.len();
                let px = &self.x;
                let py = &self.y;

                let mut p1 = &polygon[n - 1];
                for p2 in polygon.iter() {
                    if (*py > p1.y) != (*py > p2.y) {
                        if *px < (p2.x - p1.x) * (*py - p1.y) / (p2.y - p1.y) + p1.x {
                            inside = !inside;
                        }
                    }
                    p1 = p2;
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

macro_rules! impl_rotate_around_point {
    ($t:ty) => {
        impl Vector2<$t> {
            pub fn rotate_around_point(&mut self, angle: $t, origin: Vector2<$t>) -> Self {
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

            pub fn rotate_around_point_degrees(&mut self, angle: $t, origin: Vector2<$t>) -> Self {
                self.rotate_around_point(angle.to_radians(), origin)
            }
        }
    };
}

impl_rotate_around_point!(f32);
impl_rotate_around_point!(f64);
