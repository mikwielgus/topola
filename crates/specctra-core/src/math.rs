// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::Sub;
use geo_types::geometry::Point;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub pos: Point,
    pub r: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointWithRotation {
    pub pos: Point,
    pub rot: f64,
}

impl Circle {
    /// Calculate the point that lies on the circle at angle `phi`,
    /// relative to coordinate axes.
    ///
    /// `phi` is the angle given in radians starting at `(r, 0)`.
    pub fn position_at_angle(&self, phi: f64) -> Point {
        geo_types::point! {
            x: self.pos.0.x + self.r * phi.cos(),
            y: self.pos.0.y + self.r * phi.sin()
        }
    }

    /// The (x,y) axis aligned bounding box for this circle.
    #[cfg(feature = "rstar")]
    pub fn bbox(&self, margin: f64) -> rstar::AABB<[f64; 2]> {
        let r = self.r + margin;
        rstar::AABB::from_corners(
            [self.pos.0.x - r, self.pos.0.y - r],
            [self.pos.0.x + r, self.pos.0.y + r],
        )
    }
}

impl Sub for Circle {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            pos: self.pos - other.pos,
            r: self.r,
        }
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
