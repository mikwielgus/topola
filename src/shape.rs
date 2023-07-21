use geo::{Point, EuclideanDistance};
use rstar::{RTreeObject, AABB};

use crate::{weight::{TaggedWeight, DotWeight}, math::Circle};

#[derive(PartialEq)]
pub struct Shape {
    pub weight: TaggedWeight,
    pub dot_neighbor_weights: Vec<DotWeight>,
    pub core_pos: Option<Point>,
}

impl Shape {
    pub fn envelope(&self) -> AABB<[f64; 2]> {
        match self.weight {
            TaggedWeight::Dot(dot) => {
                return AABB::from_corners(
                    [dot.circle.pos.x() - dot.circle.r, dot.circle.pos.y() - dot.circle.r],
                    [dot.circle.pos.x() + dot.circle.r, dot.circle.pos.y() + dot.circle.r]
                );
            },
            TaggedWeight::Seg(..) | TaggedWeight::Bend(..) => {
                // TODO: Take widths into account.

                let points: Vec<[f64; 2]> = self.dot_neighbor_weights.iter()
                    .map(|neighbor| [neighbor.circle.pos.x(), neighbor.circle.pos.y()])
                    .collect();
                return AABB::<[f64; 2]>::from_points(&points);
            },
        }
    }

    pub fn circle(&self) -> Option<Circle> {
        match self.weight {
            TaggedWeight::Dot(dot) => Some(dot.circle),
            TaggedWeight::Seg(seg) => None,
            TaggedWeight::Bend(bend) => {
                let r = self.dot_neighbor_weights[0].circle.pos.euclidean_distance(&self.core_pos.unwrap());
                Some(Circle {
                    pos: self.core_pos.unwrap(),
                    r,
                })
            }
        }
    }

    pub fn width(&self) -> f64 {
        match self.weight {
            TaggedWeight::Dot(dot) => dot.circle.r * 2.0,
            TaggedWeight::Seg(seg) => seg.width,
            TaggedWeight::Bend(bend) => self.dot_neighbor_weights[0].circle.r * 2.0,
        }
    }

    pub fn weight(&self) -> TaggedWeight {
        return self.weight;
    }
}

impl RTreeObject for Shape {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return self.envelope();
    }
}
