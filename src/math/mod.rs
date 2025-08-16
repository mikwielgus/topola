// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::algorithm::line_measures::{Distance, Euclidean};
use geo::{point, Line, Point};
pub use specctra_core::math::{Circle, PointWithRotation};

mod cyclic_search;
pub use cyclic_search::*;

mod line;
pub use line::*;

mod polygon_tangents;
pub use polygon_tangents::*;

mod bitangents;
pub use bitangents::*;

mod tunnel;
pub use tunnel::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationSense {
    Counterclockwise,
    Clockwise,
}

impl core::ops::Neg for RotationSense {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            RotationSense::Counterclockwise => RotationSense::Clockwise,
            RotationSense::Clockwise => RotationSense::Counterclockwise,
        }
    }
}

impl RotationSense {
    /// move `pos` by `step` along `self` assuming the list of positions is ordered CCW.
    pub fn step_ccw(self, pos: usize, len: usize, mut step: usize) -> usize {
        step %= len;
        (match self {
            RotationSense::Counterclockwise => pos + step,
            RotationSense::Clockwise => len + pos - step,
        }) % len
    }
}

/// Calculates the intersection of two circles, `circle1` and `circle2`.
///
/// Returns a `Vec` holding zero, one, or two calculated intersection points,
/// depending on how many exist.
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

/// Calculate the intersection between circle `circle` and line segment `segment`.
///
/// Returns a `Vec` holding zero, one, or two calculated intersection points,
/// depending on how many exist.
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

/// Returns `true` the point `p` is between the supporting lines of vectors
/// `from` and `to`.
pub fn between_vectors(p: Point, from: Point, to: Point) -> bool {
    let cross = perp_dot_product(from, to);
    between_vectors_cached(p, from, to, cross)
}

fn between_vectors_cached(p: Point, from: Point, to: Point, cross: f64) -> bool {
    if cross > 0.0 {
        perp_dot_product(from, p) >= 0.0 && perp_dot_product(p, to) >= 0.0
    } else if cross < 0.0 {
        perp_dot_product(from, p) >= 0.0 || perp_dot_product(p, to) >= 0.0
    } else {
        false
    }
}

/// Calculates the (directed) angle between the positive X axis and vector `vector`.
///
/// The result is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
pub fn vector_angle(vector: Point) -> f64 {
    vector.y().atan2(vector.x())
}

/// Calculates the (directed) angle between vectors `v1` and `v2`.
///
/// The result is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
pub fn angle_between(v1: Point, v2: Point) -> f64 {
    perp_dot_product(v1, v2).atan2(dot_product(v1, v2))
}

/// Calculates the perp dot product of vectors `start - reference` and `stop - reference`.
pub fn seq_perp_dot_product(start: Point, stop: Point, reference: Point) -> f64 {
    let dx1 = stop.x() - start.x();
    let dy1 = stop.y() - start.y();
    let dx2 = reference.x() - stop.x();
    let dy2 = reference.y() - stop.y();
    perp_dot_product((dx1, dy1).into(), (dx2, dy2).into())
}

/// Calculates the dot product of vectors `v1` and `v2`.
pub fn dot_product(v1: Point, v2: Point) -> f64 {
    v1.x() * v2.x() + v1.y() * v2.y()
}

/// Calculates the perp dot product of vectors `v1` and `v2`.
///
/// This is defined as the dot product of `v1` rotated counterclockwise by 90
/// degrees and `v2`. This is the same as the magnitude of the cross product of `v1`
/// with `v2`.
///
/// It is not uncommon in codebases with planar geometry to call the perp dot
/// product simply "cross product", ignoring the distinction between vector
/// and its magnitude, since the resulting vector is always perpendicular to
/// the plane anyway.
pub fn perp_dot_product(v1: Point, v2: Point) -> f64 {
    // catch numerical rounding errors
    if approx::relative_eq!(v1.x(), v2.x()) && approx::relative_eq!(v1.y(), v2.y()) {
        return 0.0;
    }

    v1.x() * v2.y() - v1.y() * v2.x()
}
