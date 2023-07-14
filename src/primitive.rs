use geo::{Point, EuclideanDistance};
use rstar::{RTreeObject, AABB};

use crate::{weight::{Weight, DotWeight}, math::Circle};

#[derive(PartialEq)]
pub struct Primitive {
    pub weight: Weight,
    pub dot_neighbor_weights: Vec<DotWeight>,
    pub around_weight: Option<Weight>,
    pub focus: Option<Point>,
}

impl Primitive {
    pub fn envelope(&self) -> AABB<[f64; 2]> {
        match self.weight {
            Weight::Dot(dot) => {
                return AABB::from_corners(
                    [dot.circle.pos.x() - dot.circle.r, dot.circle.pos.y() - dot.circle.r],
                    [dot.circle.pos.x() + dot.circle.r, dot.circle.pos.y() + dot.circle.r]
                );
            },
            Weight::Seg(..) | Weight::Bend(..) => {
                // TODO: Take widths into account.

                let points: Vec<[f64; 2]> = self.dot_neighbor_weights.iter()
                    .map(|neighbor| [neighbor.circle.pos.x(), neighbor.circle.pos.y()])
                    .collect();
                return AABB::<[f64; 2]>::from_points(&points);
            },
            Weight::EndRef(..) | Weight::AroundRef(..) => unreachable!(),
        }
    }

    pub fn circle(&self) -> Option<Circle> {
        match self.weight {
            Weight::Dot(dot) => Some(dot.circle),
            Weight::Seg(seg) => None,
            Weight::Bend(bend) => {
                let r = self.dot_neighbor_weights[0].circle.pos.euclidean_distance(&self.focus.unwrap());
                Some(Circle {
                    pos: self.focus.unwrap(),
                    r,
                })
            }
            Weight::EndRef(..) | Weight::AroundRef(..) => unreachable!(),
        }
    }

    pub fn width(&self) -> f64 {
        match self.weight {
            Weight::Dot(dot) => dot.circle.r * 2.0,
            Weight::Seg(seg) => seg.width,
            Weight::Bend(bend) => self.dot_neighbor_weights[0].circle.r * 2.0,
            Weight::EndRef(..) | Weight::AroundRef(..) => unreachable!(),
        }
    }

    pub fn weight(&self) -> Weight {
        return self.weight;
    }
}

impl RTreeObject for Primitive {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return self.envelope();
    }
}
