// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::{cmp, ops};
use geo::algorithm::line_measures::{Distance, Euclidean};
use geo::{geometry::Point, point, Line};
pub use specctra_core::math::{Circle, PointWithRotation};

mod tangents;
pub use tangents::*;

pub fn intersect_circles(circle1: &Circle, circle2: &Circle) -> Vec<Point> {
    let delta = circle2.pos - circle1.pos;
    let d = Euclidean::distance(&circle2.pos, &circle1.pos);

    if d > circle1.r + circle2.r {
        // No intersection.
        return vec![];
    }

    if d < (circle2.r - circle1.r).abs() {
        // One contains the other.
        return vec![];
    }

    // Distance from `circle1.pos` to the intersection of the diagonals.
    let a = (circle1.r * circle1.r - circle2.r * circle2.r + d * d) / (2.0 * d);

    // Intersection of the diagonals.
    let p = circle1.pos + delta * (a / d);
    let h = (circle1.r * circle1.r - a * a).sqrt();

    if h == 0.0 {
        return [p].into();
    }

    let r = point! {x: -delta.x(), y: delta.y()} * (h / d);

    [p + r, p - r].into()
}

pub fn intersect_circle_segment(circle: &Circle, segment: &Line) -> Vec<Point> {
    let delta: Point = segment.delta().into();
    let from = segment.start_point();
    let to = segment.end_point();
    let epsilon = 1e-9;
    let interval01 = 0.0..=1.0;

    let a = delta.dot(delta);
    let b =
        2.0 * (delta.x() * (from.x() - circle.pos.x()) + delta.y() * (from.y() - circle.pos.y()));
    let c = circle.pos.dot(circle.pos) + from.dot(from)
        - 2.0 * circle.pos.dot(from)
        - circle.r * circle.r;
    let discriminant = b * b - 4.0 * a * c;

    if a.abs() < epsilon || discriminant < 0.0 {
        return [].into();
    }

    if discriminant == 0.0 {
        let u = -b / (2.0 * a);

        return if interval01.contains(&u) {
            vec![from + (to - from) * -b / (2.0 * a)]
        } else {
            vec![]
        };
    }

    let mut v = vec![];

    let u1 = (-b + discriminant.sqrt()) / (2.0 * a);

    if interval01.contains(&u1) {
        v.push(from + (to - from) * u1);
    }

    let u2 = (-b - discriminant.sqrt()) / (2.0 * a);

    if interval01.contains(&u2) {
        v.push(from + (to - from) * u2);
    }

    v
}

pub fn between_vectors(p: Point, from: Point, to: Point) -> bool {
    let cross = cross_product(from, to);

    if cross > 0.0 {
        cross_product(from, p) >= 0.0 && cross_product(p, to) >= 0.0
    } else if cross < 0.0 {
        cross_product(from, p) >= 0.0 || cross_product(p, to) >= 0.0
    } else {
        false
    }
}

/// An angle that is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
#[derive(Clone, Copy, Debug)]
pub struct NormalizedAngle(f64);

impl cmp::PartialOrd for NormalizedAngle {
    #[inline(always)]
    fn partial_cmp(&self, oth: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(oth))
    }
}

impl cmp::Ord for NormalizedAngle {
    #[inline(always)]
    fn cmp(&self, oth: &Self) -> cmp::Ordering {
        self.0.total_cmp(&oth.0)
    }
}

impl cmp::PartialEq for NormalizedAngle {
    fn eq(&self, oth: &Self) -> bool {
        self.cmp(oth) == cmp::Ordering::Equal
    }
}

impl cmp::Eq for NormalizedAngle {}

impl NormalizedAngle {
    pub const ZERO: Self = Self(0.0);

    fn normalize_single_step(mut angle: f64) -> Self {
        use core::f64::consts::{PI, TAU};
        if !(angle.is_nan() || angle.is_infinite()) {
            if angle <= -PI {
                angle += TAU;
            }
            if angle > PI {
                angle -= TAU;
            }
            assert!((-PI..PI).contains(&(-angle)));
        }
        Self(angle)
    }

    /// Computes the (directed) angle between the positive X axis and the vector.
    #[inline]
    pub fn atan2(pt: Point) -> Self {
        NormalizedAngle(pt.0.x.atan2(pt.0.y))
    }

    #[must_use]
    pub fn non_negative(self) -> f64 {
        let mut angle = self.0;
        if angle < 0.0 {
            angle + core::f64::consts::TAU
        } else {
            angle
        }
    }

    /// Rotate this angle by 180°
    #[must_use]
    pub fn flip(self) -> Self {
        Self::normalize_single_step(self.0 + core::f64::consts::PI)
    }
}

impl From<f64> for NormalizedAngle {
    fn from(mut angle: f64) -> Self {
        use core::f64::consts::{PI, TAU};
        if !(angle.is_nan() || angle.is_infinite()) {
            while angle <= -PI {
                angle += TAU;
            }
            while angle > PI {
                angle -= TAU;
            }
            debug_assert!((-PI..PI).contains(&(-angle)));
        }
        Self(angle)
    }
}

impl ops::Deref for NormalizedAngle {
    type Target = f64;

    #[inline(always)]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl ops::Add for NormalizedAngle {
    type Output = Self;

    fn add(self, oth: Self) -> Self {
        Self::normalize_single_step(self.0 + oth.0)
    }
}

impl ops::AddAssign for NormalizedAngle {
    fn add_assign(&mut self, oth: Self) {
        *self = Self::normalize_single_step(self.0 + oth.0);
    }
}

impl ops::Sub for NormalizedAngle {
    type Output = Self;

    fn sub(self, oth: Self) -> Self {
        Self::normalize_single_step(self.0 - oth.0)
    }
}

impl ops::SubAssign for NormalizedAngle {
    fn sub_assign(&mut self, oth: Self) {
        *self = Self::normalize_single_step(self.0 - oth.0);
    }
}

impl ops::MulAssign<f64> for NormalizedAngle {
    fn mul_assign(&mut self, oth: f64) {
        *self = (self.0 * oth).into();
    }
}

/// Computes the (directed) angle between two vectors.
///
/// The result is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
pub fn angle_between(v1: Point, v2: Point) -> NormalizedAngle {
    NormalizedAngle::atan2(geo::point! {
        x: dot_product(v1, v2),
        y: cross_product(v1, v2)
    })
}

pub fn seq_cross_product(start: Point, stop: Point, reference: Point) -> f64 {
    let dx1 = stop.x() - start.x();
    let dy1 = stop.y() - start.y();
    let dx2 = reference.x() - stop.x();
    let dy2 = reference.y() - stop.y();
    cross_product((dx1, dy1).into(), (dx2, dy2).into())
}

pub fn dot_product(v1: Point, v2: Point) -> f64 {
    v1.x() * v2.x() + v1.y() * v2.y()
}

pub fn cross_product(v1: Point, v2: Point) -> f64 {
    v1.x() * v2.y() - v1.y() * v2.x()
}
