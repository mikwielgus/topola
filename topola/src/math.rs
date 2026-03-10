// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::{Add, Constructor, From, Into, Sub};

#[derive(Add, Clone, Constructor, Copy, Debug, From, Into, Sub)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

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
