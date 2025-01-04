// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

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

/// Computes the (directed) angle between the positive X axis and the vector.
///
/// The result is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
pub fn vector_angle(vector: Point) -> f64 {
    vector.y().atan2(vector.x())
}

/// Computes the (directed) angle between two vectors.
///
/// The result is measured counterclockwise and normalized into range (-pi, pi] (like atan2).
pub fn angle_between(v1: Point, v2: Point) -> f64 {
    cross_product(v1, v2).atan2(dot_product(v1, v2))
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
