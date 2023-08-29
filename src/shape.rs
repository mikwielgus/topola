use enum_as_inner::EnumAsInner;
use geo::{point, polygon, EuclideanDistance, Intersects, Point, Polygon, Rotate};
use rstar::{RTreeObject, AABB};


use crate::math::{self, Circle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotShape {
    pub c: Circle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegShape {
    pub from: Point,
    pub to: Point,
    pub width: f64,
}

impl SegShape {
    fn polygon(&self) -> Polygon {
        let tangent_vector = self.to - self.from;
        let tangent_vector_norm = tangent_vector.euclidean_distance(&point! {x: 0.0, y: 0.0});
        let unit_tangent_vector = tangent_vector / tangent_vector_norm;

        let normal = unit_tangent_vector.rotate_around_point(-90., point! {x: 0.0, y: 0.0});

        let p1 = self.from - normal * (self.width / 2.);
        let p2 = self.from + normal * (self.width / 2.);
        let p3 = self.to + normal * (self.width / 2.);
        let p4 = self.to - normal * (self.width / 2.);

        polygon![p1.0, p2.0, p3.0, p4.0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendShape {
    pub from: Point,
    pub to: Point,
    pub c: Circle,
    pub width: f64,
}

impl BendShape {
    pub fn inner_circle(&self) -> Circle {
        self.c
    }

    pub fn circle(&self) -> Circle {
        Circle {
            pos: self.c.pos,
            r: self.c.r + self.width / 2.0,
        }
    }

    pub fn outer_circle(&self) -> Circle {
        Circle {
            pos: self.c.pos,
            r: self.c.r + self.width,
        }
    }

    fn between_ends(&self, point: Point) -> bool {
        math::between_vectors(
            point - self.c.pos,
            self.from - self.c.pos,
            self.to - self.c.pos,
        )
    }
}

#[derive(Debug, EnumAsInner, PartialEq)]
pub enum Shape {
    // Intentionally in different order to reorder `self.intersects(...)` properly.
    Dot(DotShape),
    Seg(SegShape),
    Bend(BendShape),
}

impl Shape {
    pub fn principal_point(&self) -> Point {
        match self {
            Shape::Dot(dot) => dot.c.pos,
            Shape::Seg(seg) => seg.from,
            Shape::Bend(bend) => bend.from,
        }
    }

    pub fn center(&self) -> Point {
        match self {
            Shape::Dot(dot) => dot.c.pos,
            Shape::Seg(seg) => (seg.from + seg.to) / 2.0,
            Shape::Bend(bend) => {
                let sum = (bend.from - bend.c.pos) + (bend.to - bend.c.pos);
                bend.c.pos + (sum / sum.euclidean_distance(&point! {x: 0.0, y: 0.0})) * bend.c.r
            }
        }
    }

    fn priority(&self) -> i64 {
        match self {
            Shape::Dot(..) => 3,
            Shape::Bend(..) => 2,
            Shape::Seg(..) => 1,
        }
    }

    pub fn intersects(&self, other: &Shape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(self);
        }

        match self {
            Shape::Dot(dot) => match other {
                Shape::Dot(other) => {
                    dot.c.pos.euclidean_distance(&other.c.pos) < dot.c.r + other.c.r
                }
                Shape::Seg(other) => dot.c.pos.euclidean_distance(&other.polygon()) < dot.c.r,
                Shape::Bend(other) => {
                    for point in math::intersect_circles(&dot.c, &other.inner_circle()) {
                        if other.between_ends(point) {
                            return true;
                        }
                    }

                    for point in math::intersect_circles(&dot.c, &other.outer_circle()) {
                        if other.between_ends(point) {
                            return true;
                        }
                    }

                    false
                }
            },
            Shape::Seg(seg) => match other {
                Shape::Dot(..) => unreachable!(),
                Shape::Seg(other) => seg.polygon().intersects(&other.polygon()),
                Shape::Bend(other) => {
                    for segment in seg.polygon().exterior().lines() {
                        let inner_circle = other.inner_circle();
                        let outer_circle = other.outer_circle();

                        for point in math::intersect_circle_segment(&inner_circle, &segment) {
                            if other.between_ends(point) {
                                return true;
                            }
                        }

                        for point in math::intersect_circle_segment(&outer_circle, &segment) {
                            if other.between_ends(point) {
                                return true;
                            }
                        }
                    }

                    false
                }
            },
            Shape::Bend(bend) => match other {
                Shape::Dot(..) | Shape::Seg(..) => unreachable!(),
                Shape::Bend(other) => {
                    for point in
                        math::intersect_circles(&bend.inner_circle(), &other.inner_circle())
                    {
                        if bend.between_ends(point) && other.between_ends(point) {
                            return true;
                        }
                    }

                    for point in
                        math::intersect_circles(&bend.inner_circle(), &other.outer_circle())
                    {
                        if bend.between_ends(point) && other.between_ends(point) {
                            return true;
                        }
                    }

                    for point in
                        math::intersect_circles(&bend.outer_circle(), &other.inner_circle())
                    {
                        if bend.between_ends(point) && other.between_ends(point) {
                            return true;
                        }
                    }

                    for point in
                        math::intersect_circles(&bend.outer_circle(), &other.outer_circle())
                    {
                        if bend.between_ends(point) && other.between_ends(point) {
                            return true;
                        }
                    }

                    false
                }
            },
        }
    }

    pub fn envelope(&self) -> AABB<[f64; 2]> {
        match self {
            Shape::Dot(dot) => AABB::from_corners(
                [dot.c.pos.x() - dot.c.r, dot.c.pos.y() - dot.c.r],
                [dot.c.pos.x() + dot.c.r, dot.c.pos.y() + dot.c.r],
            ),
            Shape::Seg(seg) => {
                let points: Vec<[f64; 2]> = seg
                    .polygon()
                    .exterior()
                    .points()
                    .map(|p| [p.x(), p.y()])
                    .collect();
                AABB::<[f64; 2]>::from_points(points.iter())
            }
            Shape::Bend(bend) => {
                let halfwidth = bend.c.r + bend.width;
                AABB::from_corners(
                    [bend.c.pos.x() - halfwidth, bend.c.pos.y() - halfwidth],
                    [bend.c.pos.x() + halfwidth, bend.c.pos.y() + halfwidth],
                )
            }
        }
    }

    pub fn width(&self) -> f64 {
        match self {
            Shape::Dot(dot) => dot.c.r * 2.0,
            Shape::Seg(seg) => seg.width,
            Shape::Bend(bend) => bend.width,
        }
    }
}

impl RTreeObject for Shape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return self.envelope();
    }
}
