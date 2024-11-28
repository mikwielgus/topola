use core::ops::Sub;
use geo::geometry::Point;
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
