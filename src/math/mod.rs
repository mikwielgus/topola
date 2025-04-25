// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::algorithm::line_measures::{Distance, Euclidean};
use geo::{geometry::Point, point, Line};
pub use specctra_core::math::{Circle, PointWithRotation};

mod tangents;
pub use tangents::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineIntersection {
    Empty,
    Overlapping,
    Point(Point),
}

/// A line in the normal form: `x0*y + y0*y + offset = 0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalLine {
    pub x: f64,
    pub y: f64,
    pub offset: f64,
}

impl From<Line> for NormalLine {
    fn from(l: Line) -> Self {
        // the normal vector is perpendicular to the line
        let normal = geo::point! {
            x: l.dy(),
            y: -l.dx(),
        };
        Self {
            x: normal.0.x,
            y: normal.0.y,
            offset: -perp_dot_product(l.end.into(), l.start.into()),
        }
    }
}

impl NormalLine {
    pub fn evaluate_at(&self, pt: Point) -> f64 {
        self.x * pt.x() + self.y * pt.y() + self.offset
    }

    pub fn angle(&self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn make_normal_unit(&mut self) {
        let normal_len = self.y.hypot(self.x);
        if normal_len > (f64::EPSILON * 16.0) {
            self.x /= normal_len;
            self.y /= normal_len;
            self.offset /= normal_len;
        }
    }

    /// Calculate the intersection between two lines.
    pub fn intersects(&self, b: &Self) -> LineIntersection {
        const ALMOST_ZERO: f64 = f64::EPSILON * 16.0;
        let (mut a, mut b) = (*self, *b);
        let _ = (a.make_normal_unit(), b.make_normal_unit());
        let apt = geo::point! { x: a.x, y: a.y };
        let bpt = geo::point! { x: b.x, y: b.y };
        let det = perp_dot_product(apt, bpt);
        let rpx = -b.y * a.offset + a.y * b.offset;
        let rpy = b.x * a.offset - a.x * b.offset;

        if det.abs() > ALMOST_ZERO {
            LineIntersection::Point(geo::point! { x: rpx, y: rpy } / det)
        } else if rpx.abs() <= ALMOST_ZERO && rpy.abs() <= ALMOST_ZERO {
            LineIntersection::Overlapping
        } else {
            LineIntersection::Empty
        }
    }

    /// Project the point `pt` onto this line, and generate a new line which is orthogonal
    /// to `self`, and goes through `pt`.
    #[inline]
    pub fn orthogonal_through(&self, pt: &Point) -> Self {
        Self {
            // recover the original parallel vector
            x: -self.y,
            y: self.x,
            offset: -self.x * pt.0.y + self.y * pt.0.x,
        }
    }

    pub fn segment_interval(&self, line: &Line) -> core::ops::RangeInclusive<f64> {
        // recover the original parallel vector
        let parv = geo::point! {
            x: -self.y,
            y: self.x,
        };
        dot_product(parv, line.start.into())..=dot_product(parv, line.end.into())
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
    v1.x() * v2.y() - v1.y() * v2.x()
}
