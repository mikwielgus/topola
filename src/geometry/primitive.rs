use enum_dispatch::enum_dispatch;
use geo::{point, polygon, Contains, EuclideanDistance, Intersects, Point, Polygon, Rotate};
use rstar::{RTreeObject, AABB};

use crate::{
    geometry::shape::ShapeTrait,
    math::{self, Circle},
};

#[enum_dispatch]
pub trait PrimitiveShapeTrait: ShapeTrait {
    fn priority(&self) -> usize;
    fn inflate(&self, margin: f64) -> PrimitiveShape;
    fn intersects(&self, other: &PrimitiveShape) -> bool;
    fn envelope(&self, margin: f64) -> AABB<[f64; 2]>;
    fn width(&self) -> f64;
    fn length(&self) -> f64;

    fn envelope_3d(&self, margin: f64, layer: usize) -> AABB<[f64; 3]> {
        let envelope = self.envelope(margin);
        AABB::from_corners(
            [envelope.lower()[0], envelope.lower()[1], layer as f64],
            [envelope.upper()[0], envelope.upper()[1], layer as f64],
        )
    }

    fn full_height_envelope_3d(&self, margin: f64, layer_count: usize) -> AABB<[f64; 3]> {
        let envelope = self.envelope(margin);
        AABB::from_corners(
            [envelope.lower()[0], envelope.lower()[1], 0.0],
            [
                envelope.upper()[0],
                envelope.upper()[1],
                (layer_count - 1) as f64,
            ],
        )
    }
}

#[enum_dispatch(ShapeTrait, PrimitiveShapeTrait)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveShape {
    // Intentionally in different order to reorder `self.intersects(...)` properly.
    Dot(DotShape),
    Seg(SegShape),
    Bend(BendShape),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotShape {
    pub circle: Circle,
}

impl ShapeTrait for DotShape {
    fn center(&self) -> Point {
        self.circle.pos
    }

    fn contains_point(&self, p: Point) -> bool {
        p.euclidean_distance(&self.circle.pos) <= self.circle.r
    }
}

impl PrimitiveShapeTrait for DotShape {
    fn priority(&self) -> usize {
        3
    }

    fn inflate(&self, margin: f64) -> PrimitiveShape {
        PrimitiveShape::Dot(DotShape {
            circle: Circle {
                pos: self.circle.pos,
                r: self.circle.r + margin,
            },
        })
    }

    fn intersects(&self, other: &PrimitiveShape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&PrimitiveShape::from(*self));
        }

        match other {
            PrimitiveShape::Dot(other) => {
                self.circle.pos.euclidean_distance(&other.circle.pos)
                    < self.circle.r + other.circle.r
            }
            PrimitiveShape::Seg(other) => {
                self.circle.pos.euclidean_distance(&other.polygon()) < self.circle.r
            }
            PrimitiveShape::Bend(other) => {
                for point in math::intersect_circles(&self.circle, &other.inner_circle()) {
                    if other.between_ends(point) {
                        return true;
                    }
                }

                for point in math::intersect_circles(&self.circle, &other.outer_circle()) {
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
                self.circle.pos.x() - self.circle.r - margin,
                self.circle.pos.y() - self.circle.r - margin,
            ],
            [
                self.circle.pos.x() + self.circle.r + margin,
                self.circle.pos.y() + self.circle.r + margin,
            ],
        )
    }

    fn width(&self) -> f64 {
        self.circle.r * 2.0
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
    fn center(&self) -> Point {
        (self.from + self.to) / 2.0
    }

    fn contains_point(&self, p: Point) -> bool {
        self.polygon().contains(&p)
    }
}

impl PrimitiveShapeTrait for SegShape {
    fn priority(&self) -> usize {
        2
    }

    fn inflate(&self, margin: f64) -> PrimitiveShape {
        PrimitiveShape::Seg(SegShape {
            from: self.from,
            to: self.to,
            width: self.width + 2.0 * margin,
        })
    }

    fn intersects(&self, other: &PrimitiveShape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&PrimitiveShape::from(*self));
        }

        match other {
            PrimitiveShape::Dot(..) => unreachable!(),
            PrimitiveShape::Seg(other) => self.polygon().intersects(&other.polygon()),
            PrimitiveShape::Bend(other) => {
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
    pub inner_circle: Circle,
    pub width: f64,
}

impl BendShape {
    pub fn inner_circle(&self) -> Circle {
        self.inner_circle
    }

    pub fn circle(&self) -> Circle {
        Circle {
            pos: self.inner_circle.pos,
            r: self.inner_circle.r + self.width / 2.0,
        }
    }

    pub fn outer_circle(&self) -> Circle {
        Circle {
            pos: self.inner_circle.pos,
            r: self.inner_circle.r + self.width,
        }
    }

    pub fn between_ends(&self, point: Point) -> bool {
        math::between_vectors(
            point - self.inner_circle.pos,
            self.from - self.inner_circle.pos,
            self.to - self.inner_circle.pos,
        )
    }
}

impl ShapeTrait for BendShape {
    fn center(&self) -> Point {
        let sum = (self.from - self.inner_circle.pos) + (self.to - self.inner_circle.pos);
        self.inner_circle.pos
            + (sum / sum.euclidean_distance(&point! {x: 0.0, y: 0.0})) * self.inner_circle.r
    }

    fn contains_point(&self, p: Point) -> bool {
        let d = p.euclidean_distance(&self.inner_circle.pos);
        self.between_ends(p) && d >= self.inner_circle().r && d <= self.outer_circle().r
    }
}

impl PrimitiveShapeTrait for BendShape {
    fn priority(&self) -> usize {
        1
    }

    fn inflate(&self, margin: f64) -> PrimitiveShape {
        PrimitiveShape::Bend(BendShape {
            from: self.from, // TODO: Is not inflated for now.
            to: self.to,     // TODO: Is not inflated for now.
            inner_circle: Circle {
                pos: self.inner_circle.pos,
                r: self.inner_circle.r - margin,
            },
            width: self.width + 2.0 * margin,
        })
    }

    fn intersects(&self, other: &PrimitiveShape) -> bool {
        if self.priority() < other.priority() {
            return other.intersects(&PrimitiveShape::from(*self));
        }

        match other {
            PrimitiveShape::Dot(..) | PrimitiveShape::Seg(..) => unreachable!(),
            PrimitiveShape::Bend(other) => {
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

    fn envelope(&self, _margin: f64) -> AABB<[f64; 2]> {
        let halfwidth = self.inner_circle.r + self.width;
        AABB::from_corners(
            [
                self.inner_circle.pos.x() - halfwidth,
                self.inner_circle.pos.y() - halfwidth,
            ],
            [
                self.inner_circle.pos.x() + halfwidth,
                self.inner_circle.pos.y() + halfwidth,
            ],
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

impl RTreeObject for PrimitiveShape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        PrimitiveShapeTrait::envelope(self, 0.0)
    }
}
