use enum_dispatch::enum_dispatch;
use geo::{point, polygon, EuclideanDistance, Intersects, Point, Polygon, Rotate};
use rstar::{RTreeObject, AABB};

use crate::math::{self, Circle};

#[enum_dispatch]
pub trait ShapeTrait {
    fn priority(&self) -> u64;
    fn center(&self) -> Point;
    fn intersects(&self, other: &Shape) -> bool;
    fn envelope(&self) -> AABB<[f64; 2]>;
    fn width(&self) -> f64;
}

#[enum_dispatch(ShapeTrait)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    // Intentionally in different order to reorder `self.intersects(...)` properly.
    Dot(DotShape),
    Seg(SegShape),
    Bend(BendShape),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotShape {
    pub c: Circle,
}

impl ShapeTrait for DotShape {
    fn priority(&self) -> u64 {
        3
    }

    fn center(&self) -> Point {
        self.c.pos
    }

    fn intersects(&self, other: &Shape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&Shape::from(*self));
        }

        match other {
            Shape::Dot(other) => self.c.pos.euclidean_distance(&other.c.pos) < self.c.r + other.c.r,
            Shape::Seg(other) => self.c.pos.euclidean_distance(&other.polygon()) < self.c.r,
            Shape::Bend(other) => {
                for point in math::intersect_circles(&self.c, &other.inner_circle()) {
                    if other.between_ends(point) {
                        return true;
                    }
                }

                for point in math::intersect_circles(&self.c, &other.outer_circle()) {
                    if other.between_ends(point) {
                        return true;
                    }
                }

                false
            }
        }
    }

    fn envelope(&self) -> AABB<[f64; 2]> {
        AABB::from_corners(
            [self.c.pos.x() - self.c.r, self.c.pos.y() - self.c.r],
            [self.c.pos.x() + self.c.r, self.c.pos.y() + self.c.r],
        )
    }

    fn width(&self) -> f64 {
        self.c.r * 2.0
    }
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

impl ShapeTrait for SegShape {
    fn priority(&self) -> u64 {
        2
    }

    fn center(&self) -> Point {
        (self.from + self.to) / 2.0
    }

    fn intersects(&self, other: &Shape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&Shape::from(*self));
        }

        match other {
            Shape::Dot(..) => unreachable!(),
            Shape::Seg(other) => self.polygon().intersects(&other.polygon()),
            Shape::Bend(_other) => {
                /*for segment in self.polygon().exterior().lines() {
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
                }*/

                false
            }
        }
    }

    fn envelope(&self) -> AABB<[f64; 2]> {
        let points: Vec<[f64; 2]> = self
            .polygon()
            .exterior()
            .points()
            .map(|p| [p.x(), p.y()])
            .collect();
        AABB::<[f64; 2]>::from_points(points.iter())
    }

    fn width(&self) -> f64 {
        self.width
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

    pub fn between_ends(&self, point: Point) -> bool {
        math::between_vectors(
            point - self.c.pos,
            self.from - self.c.pos,
            self.to - self.c.pos,
        )
    }
}

impl ShapeTrait for BendShape {
    fn priority(&self) -> u64 {
        1
    }

    fn center(&self) -> Point {
        let sum = (self.from - self.c.pos) + (self.to - self.c.pos);
        self.c.pos + (sum / sum.euclidean_distance(&point! {x: 0.0, y: 0.0})) * self.c.r
    }

    fn intersects(&self, other: &Shape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&Shape::from(*self));
        }

        match other {
            Shape::Dot(..) | Shape::Seg(..) => unreachable!(),
            Shape::Bend(other) => {
                for point in math::intersect_circles(&self.inner_circle(), &other.inner_circle()) {
                    if self.between_ends(point) && other.between_ends(point) {
                        return true;
                    }
                }

                for point in math::intersect_circles(&self.inner_circle(), &other.outer_circle()) {
                    if self.between_ends(point) && other.between_ends(point) {
                        return true;
                    }
                }

                for point in math::intersect_circles(&self.outer_circle(), &other.inner_circle()) {
                    if self.between_ends(point) && other.between_ends(point) {
                        return true;
                    }
                }

                for point in math::intersect_circles(&self.outer_circle(), &other.outer_circle()) {
                    if self.between_ends(point) && other.between_ends(point) {
                        return true;
                    }
                }

                false
            }
        }
    }

    fn envelope(&self) -> AABB<[f64; 2]> {
        let halfwidth = self.c.r + self.width;
        AABB::from_corners(
            [self.c.pos.x() - halfwidth, self.c.pos.y() - halfwidth],
            [self.c.pos.x() + halfwidth, self.c.pos.y() + halfwidth],
        )
    }

    fn width(&self) -> f64 {
        self.width
    }
}

impl RTreeObject for Shape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return ShapeTrait::envelope(self);
    }
}
