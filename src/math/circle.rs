// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::Sub;

use geo::{point, Distance, Euclidean, Length, Line, Point};
use serde::{Deserialize, Serialize};

use crate::math;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Circle {
    pub pos: Point,
    pub r: f64,
}

impl Circle {
    /// Calculate the point that lies on the circle at angle `phi`,
    /// relative to coordinate axes.
    ///
    /// `phi` is the angle given in radians starting at `(r, 0)`.
    pub fn position_at_angle(&self, phi: f64) -> Point {
        point! {
            x: self.pos.0.x + self.r * phi.cos(),
            y: self.pos.0.y + self.r * phi.sin()
        }
    }

    /// The (x,y) axis aligned bounding box for this circle.
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

/// Find the filleting circle of line segments `segment1` and `segment2`.
pub fn fillet_circle(segment1: &Line, segment2: &Line) -> Circle {
    // Turn segment1 delta vector counterclockwisely by 90 degrees.
    let diameter_ray = Line::new(
        segment1.end_point(),
        point! {x: segment1.end_point().x() - segment1.delta().y, y: segment1.end_point().y() + segment1.delta().x},
    );

    // Radius is the distance from the diameter line to segment2.start_point().
    let radius = (diameter_ray.delta().y * segment2.start_point().x()
        - diameter_ray.delta().x * segment2.start_point().y()
        + diameter_ray.end_point().x() * diameter_ray.start_point().y()
        - diameter_ray.end_point().y() * diameter_ray.start_point().x())
    .abs()
        / diameter_ray.length::<Euclidean>();

    let center =
        if math::perp_dot_product(Point::from(segment1.delta()), Point::from(segment2.delta()))
            >= 0.0
        {
            segment1.end_point()
                + (diameter_ray.delta() / diameter_ray.length::<Euclidean>() * radius).into()
        } else {
            segment1.end_point()
                - (diameter_ray.delta() / diameter_ray.length::<Euclidean>() * radius).into()
        };

    Circle {
        pos: center,
        r: radius,
    }
}
