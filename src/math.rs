use std::ops::Sub;
use geo::geometry::Point;

#[derive(Clone, Copy, PartialEq)]
pub struct Line {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Circle {
    pub pos: Point,
    pub r: f64,
}

impl Sub for Circle {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        //return Self{pos: Point{x: self.pos.x() - other.pos.x(), y: self.pos.y() - other.pos.y()}, r: self.r};
        return Self {pos: self.pos - other.pos, r: self.r};
    }
}

fn _tangent(center: Point, r1: f64, r2: f64) -> Line {
    let epsilon = 1e-9;
    let dr = r2 - r1;
    let norm = center.x() * center.x() + center.y() * center.y();
    let discriminant = norm - dr * dr;

    if discriminant < -epsilon {
        panic!();
    }

    let sqrt_discriminant = f64::sqrt(f64::abs(discriminant));

    Line {
        a: (center.x() * dr + center.y() * sqrt_discriminant) / norm,
        b: (center.y() * dr - center.x() * sqrt_discriminant) / norm,
        c: r1,
    }
}

fn _tangents(circle1: Circle, circle2: Circle) -> [Line; 4] {
    let mut tgs: [Line; 4] = [
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

fn cast_point_to_line(pt: Point, line: Line) -> Point {
    return (
        (line.b * (line.b * pt.x() - line.a * pt.y()) - line.a * line.c) / (line.a * line.a + line.b * line.b),
        (line.a * (-line.b * pt.x() + line.a * pt.y()) - line.b * line.c) / (line.a * line.a + line.b * line.b),
    ).into();
}

pub fn tangent_point_pairs(circle1: Circle, circle2: Circle) -> [(Point, Point); 4] {
    let tgs = _tangents(circle1, circle2);

    [
        (cast_point_to_line(circle1.pos, tgs[0]), cast_point_to_line(circle2.pos, tgs[0])),
        (cast_point_to_line(circle1.pos, tgs[1]), cast_point_to_line(circle2.pos, tgs[1])),
        (cast_point_to_line(circle1.pos, tgs[2]), cast_point_to_line(circle2.pos, tgs[2])),
        (cast_point_to_line(circle1.pos, tgs[3]), cast_point_to_line(circle2.pos, tgs[3])),
    ]
}

pub fn tangent_point_pair(circle1: Circle, cw1: Option<bool>, circle2: Circle, cw2: Option<bool>) -> (Point, Point) {
    let tangent_point_pairs = tangent_point_pairs(circle1, circle2);

    for tangent_point_pair in tangent_point_pairs {
        if let Some(cw1) = cw1 {
            let cross1 = cross_product(tangent_point_pair.0, tangent_point_pair.1, circle1.pos);

            if (cw1 && cross1 <= 0.0) || (!cw1 && cross1 >= 0.0) {
                continue;
            }
        }

        if let Some(cw2) = cw2 {
            let cross2 = cross_product(tangent_point_pair.0, tangent_point_pair.1, circle2.pos);

            if (cw2 && cross2 <= 0.0) || (!cw2 && cross2 >= 0.0) {
                continue;
            }
        }

        return tangent_point_pair;
    }

    unreachable!();
}

pub fn cross_product(start: Point, stop: Point, reference: Point) -> f64 {
    let dx1 = stop.x() - start.x();
    let dy1 = stop.y() - start.y();
    let dx2 = reference.x() - stop.x();
    let dy2 = reference.y() - stop.y();
    dx1 * dy2 - dy1 * dx2
}
