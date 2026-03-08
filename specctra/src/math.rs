// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::fmt;
use geo_types::geometry::Point;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub pos: Point,
    pub r: f64,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointWithRotation {
    pub pos: Point,
    pub rot: f64,
}

impl fmt::Debug for PointWithRotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PointWithRotation")
            .field("x", &self.pos.0.x)
            .field("y", &self.pos.0.y)
            .field("rot", &self.rot)
            .finish()
    }
}

impl Default for PointWithRotation {
    fn default() -> Self {
        Self {
            pos: (0.0, 0.0).into(),
            rot: 0.0,
        }
    }
}

impl PointWithRotation {
    pub fn from_xy(x: f64, y: f64) -> Self {
        Self {
            pos: (x, y).into(),
            rot: 0.0,
        }
    }
}
