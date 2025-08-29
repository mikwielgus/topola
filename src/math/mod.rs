// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;

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

mod circle;
pub use circle::*;

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
