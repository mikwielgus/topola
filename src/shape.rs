use geo::{Point, EuclideanDistance};
use rstar::{RTreeObject, AABB};

use crate::{weight::{TaggedWeight, DotWeight}, math::Circle};

#[derive(PartialEq)]
pub struct Shape {
    pub width: f64,
    pub from: Point,
    pub to: Point,
    pub center: Option<Point>,
}

impl Shape {
    pub fn new(width: f64, from: Point, to: Point, center: Option<Point>) -> Self {
        Shape {width, from, to, center}
    }

    pub fn envelope(&self) -> AABB<[f64; 2]> {
        if self.from == self.to {
            AABB::from_corners(
                [self.from.x() - self.width, self.from.y() - self.width],
                [self.from.x() + self.width, self.from.y() + self.width]
            )
        } else {
            // TODO: Take widths into account.
            AABB::<[f64; 2]>::from_points(&[[self.from.x(), self.from.y()],
                                            [self.to.x(), self.to.y()]])
        }
    }

    pub fn circle(&self) -> Option<Circle> {
        if let Some(center) = self.center {
            let r = self.from.euclidean_distance(&center);
            Some(Circle {
                pos: center,
                r,
            })
        } else {
            None
        }
    }
}

impl RTreeObject for Shape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return self.envelope();
    }
}
