use geo::{Point, EuclideanDistance};
use rstar::{RTreeObject, AABB};

use crate::graph::{TaggedWeight, DotWeight};
use crate::math::Circle;

#[derive(Debug, PartialEq)]
pub struct DotShape {
    pub c: Circle,
}

#[derive(Debug, PartialEq)]
pub struct SegShape {
    pub from: Point,
    pub to: Point,
    pub width: f64,
}

#[derive(Debug, PartialEq)]
pub struct BendShape {
    pub from: Point,
    pub to: Point,
    pub center: Point,
    pub width: f64,
}

#[derive(Debug, PartialEq)]
pub enum Shape {
    Dot(DotShape),
    Seg(SegShape),
    Bend(BendShape),
}

impl Shape {
    pub fn envelope(&self) -> AABB<[f64; 2]> {
        match self {
            Shape::Dot(dot) =>
                AABB::from_corners(
                    [dot.c.pos.x() - dot.c.r, dot.c.pos.y() - dot.c.r],
                    [dot.c.pos.x() + dot.c.r, dot.c.pos.y() + dot.c.r],
                ),
            Shape::Seg(seg) =>
                AABB::<[f64; 2]>::from_points(&[[seg.from.x(), seg.from.y()],
                                                [seg.to.x(), seg.to.y()]]),
            Shape::Bend(bend) =>
                AABB::<[f64; 2]>::from_points(&[[bend.from.x() - bend.width,
                                                 bend.from.y() - bend.width],
                                                [bend.to.x() + bend.width,
                                                 bend.to.y() + bend.width]]),
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
