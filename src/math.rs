use geo::{geometry::Point, point, EuclideanDistance, Line};
use std::ops::Sub;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanonicalLine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub pos: Point,
    pub r: f64,
}

impl Sub for Circle {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        return Self {
            pos: self.pos - other.pos,
            r: self.r,
        };
    }
}

fn _tangent(center: Point, r1: f64, r2: f64) -> CanonicalLine {
    let epsilon = 1e-9;
    let dr = r2 - r1;
    let norm = center.x() * center.x() + center.y() * center.y();
    let discriminant = norm - dr * dr;

    if discriminant < -epsilon {
        panic!();
    }

    let sqrt_discriminant = f64::sqrt(f64::abs(discriminant));

    CanonicalLine {
        a: (center.x() * dr + center.y() * sqrt_discriminant) / norm,
        b: (center.y() * dr - center.x() * sqrt_discriminant) / norm,
        c: r1,
    }
}

fn _tangents(circle1: Circle, circle2: Circle) -> [CanonicalLine; 4] {
    let mut tgs: [CanonicalLine; 4] = [
        _tangent((circle2 - circle1).pos, -circle1.r, -circle2.r),
        _tangent((circle2 - circle1).pos, -circle1.r, circle2.r),
        _tangent((circle2 - circle1).pos, circle1.r, -circle2.r),
        _tangent((circle2 - circle1).pos, circle1.r, circle2.r),
    ];

    for tg in tgs.iter_mut() {
        tg.c -= tg.a * circle1.pos.x() + tg.b * circle1.pos.y();
    }

    return tgs;
}

fn cast_point_to_canonical_line(pt: Point, line: CanonicalLine) -> Point {
    return (
        (line.b * (line.b * pt.x() - line.a * pt.y()) - line.a * line.c)
            / (line.a * line.a + line.b * line.b),
        (line.a * (-line.b * pt.x() + line.a * pt.y()) - line.b * line.c)
            / (line.a * line.a + line.b * line.b),
    )
        .into();
}

pub fn tangent_point_pairs(circle1: Circle, circle2: Circle) -> [(Point, Point); 4] {
    let tgs = _tangents(circle1, circle2);

    [
        (
            cast_point_to_canonical_line(circle1.pos, tgs[0]),
            cast_point_to_canonical_line(circle2.pos, tgs[0]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[1]),
            cast_point_to_canonical_line(circle2.pos, tgs[1]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[2]),
            cast_point_to_canonical_line(circle2.pos, tgs[2]),
        ),
        (
            cast_point_to_canonical_line(circle1.pos, tgs[3]),
            cast_point_to_canonical_line(circle2.pos, tgs[3]),
        ),
    ]
}

pub fn tangent_point_pair(
    circle1: Circle,
    cw1: Option<bool>,
    circle2: Circle,
    cw2: Option<bool>,
) -> (Point, Point) {
    let tangent_point_pairs = tangent_point_pairs(circle1, circle2);

    for tangent_point_pair in tangent_point_pairs {
        if let Some(cw1) = cw1 {
            let cross1 = seq_cross_product(tangent_point_pair.0, tangent_point_pair.1, circle1.pos);

            if (cw1 && cross1 <= 0.0) || (!cw1 && cross1 >= 0.0) {
                continue;
            }
        }

        if let Some(cw2) = cw2 {
            let cross2 = seq_cross_product(tangent_point_pair.0, tangent_point_pair.1, circle2.pos);

            if (cw2 && cross2 <= 0.0) || (!cw2 && cross2 >= 0.0) {
                continue;
            }
        }

        return tangent_point_pair;
    }

    unreachable!();
}

pub fn intersect_circles(circle1: &Circle, circle2: &Circle) -> Vec<Point> {
    let delta = circle2.pos - circle1.pos;
    let d = circle2.pos.euclidean_distance(&circle1.pos);

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

    if h == 0. {
        return [p].into();
    }

    let r = point! {x: -delta.x(), y: delta.y()} * (h / d);

    [p + r, p - r].into()
}

/*pub fn cast_point_to_segment(p: &Point, segment: &Line) -> Option<Point> {
    let delta = segment.end_point() - segment.start_point();
    let rel_p = *p - segment.start_point();
    let t = delta.dot(rel_p) / delta.dot(delta);

    if t < 0.0 || t > 1.0 {
        return None;
    }

    Some(segment.start_point() + delta * t)
}*/

pub fn intersect_circle_segment(circle: &Circle, segment: &Line) -> Vec<Point> {
    let delta: Point = segment.delta().into();
    let from = segment.start_point();
    let to = segment.end_point();
    let epsilon = 1e-9;

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
        return [from + (to - from) * -b / (2.0 * a)].into();
    }

    [
        from + (to - from) * (-b + discriminant.sqrt()) / (2.0 * a),
        from + (to - from) * (-b - discriminant.sqrt()) / (2.0 * a),
    ]
    .into()
}

pub fn between_vectors(p: Point, from: Point, to: Point) -> bool {
    let cross = cross_product(from, to);

    if cross >= 0. {
        cross_product(from, p) >= 0. && cross_product(p, to) >= 0.
    } else {
        cross_product(from, p) >= 0. || cross_product(p, to) >= 0.
    }
}

pub fn seq_cross_product(start: Point, stop: Point, reference: Point) -> f64 {
    let dx1 = stop.x() - start.x();
    let dy1 = stop.y() - start.y();
    let dx2 = reference.x() - stop.x();
    let dy2 = reference.y() - stop.y();
    cross_product((dx1, dy1).into(), (dx2, dy2).into())
}

pub fn cross_product(v1: Point, v2: Point) -> f64 {
    v1.x() * v2.y() - v1.y() * v2.x()
}
