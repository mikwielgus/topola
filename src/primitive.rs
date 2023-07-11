use rstar::{RTreeObject, AABB};

use crate::weight::{Weight, DotWeight};

#[derive(PartialEq)]
pub struct Primitive {
    pub weight: Weight,
    pub dot_neighbor_weights: Vec<DotWeight>,
    pub around_weight: Option<DotWeight>,
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
        }
    }

    /*pub fn weight(&self) -> Weight {
        return self.weight;
    }*/
}

impl RTreeObject for Primitive {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        return self.envelope();
    }
}
