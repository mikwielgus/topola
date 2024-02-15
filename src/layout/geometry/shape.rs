use enum_dispatch::enum_dispatch;
use geo::{point, polygon, EuclideanDistance, Intersects, Point, Polygon, Rotate};
use rstar::{RTreeObject, AABB};

use crate::math::{self, Circle};

#[enum_dispatch]
pub trait ShapeTrait {
    fn priority(&self) -> u64;
    fn inflate(&self, margin: f64) -> Shape;
    fn center(&self) -> Point;
    fn intersects(&self, other: &Shape) -> bool;
    fn envelope(&self, margin: f64) -> AABB<[f64; 2]>;
    fn width(&self) -> f64;
    fn length(&self) -> f64;
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

    fn inflate(&self, margin: f64) -> Shape {
        Shape::Dot(DotShape {
            c: Circle {
                pos: self.c.pos,
                r: self.c.r + margin,
            },
        })
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

    fn envelope(&self, margin: f64) -> AABB<[f64; 2]> {
        AABB::from_corners(
            [
                self.c.pos.x() - self.c.r - margin,
                self.c.pos.y() - self.c.r - margin,
            ],
            [
                self.c.pos.x() + self.c.r + margin,
                self.c.pos.y() + self.c.r + margin,
            ],
        )
    }

    fn width(&self) -> f64 {
        self.c.r * 2.0
    }

    fn length(&self) -> f64 {
        0.0
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

    fn inflate(&self, margin: f64) -> Shape {
        Shape::Seg(SegShape {
            from: self.from,
            to: self.to,
            width: self.width + 2.0 * margin,
        })
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
            Shape::Bend(other) => {
                for segment in self.polygon().exterior().lines() {
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
        }
    }

    fn envelope(&self, margin: f64) -> AABB<[f64; 2]> {
        let points: Vec<[f64; 2]> = self
            .polygon()
            .exterior()
            .points()
            .map(|p| [p.x(), p.y()])
            .collect();

        let aabb = AABB::<[f64; 2]>::from_points(points.iter());

        // Inflate.
        let lower = [aabb.lower()[0] - margin, aabb.lower()[1] - margin];
        let upper = [aabb.upper()[0] + margin, aabb.upper()[1] + margin];
        AABB::<[f64; 2]>::from_corners(lower, upper)
    }

    fn width(&self) -> f64 {
        self.width
    }

    fn length(&self) -> f64 {
        self.to.euclidean_distance(&self.from)
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

    fn inflate(&self, margin: f64) -> Shape {
        Shape::Bend(BendShape {
            from: self.from, // TODO: Is not inflated for now.
            to: self.to,     // TODO: Is not inflated for now.
            c: Circle {
                pos: self.c.pos,
                r: self.c.r - margin,
            },
            width: self.width + 2.0 * margin,
        })
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

    fn envelope(&self, margin: f64) -> AABB<[f64; 2]> {
        let halfwidth = self.c.r + self.width;
        AABB::from_corners(
            [self.c.pos.x() - halfwidth, self.c.pos.y() - halfwidth],
            [self.c.pos.x() + halfwidth, self.c.pos.y() + halfwidth],
        )
    }

    fn width(&self) -> f64 {
        self.width
    }

    fn length(&self) -> f64 {
        // TODO: Not valid for inflated bends, as currently `from` and `to` of these don't lie on
        // teir circles.

        // We obtain the angle from the law of cosines and multiply with radius to get the length.
        let d = self.to.euclidean_distance(&self.from);

        if d > 0.0 {
            (1.0 - d * d / (2.0 * d * d)).acos()
        } else {
            0.0
        }
    }
}

impl RTreeObject for Shape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return ShapeTrait::envelope(self, 0.0);
    }
}
