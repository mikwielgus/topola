// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::algorithm::line_measures::{Euclidean, Length};
use geo::{Centroid, Contains, Point, Polygon};
use rstar::AABB;

use crate::geometry::shape::{AccessShape, MeasureLength};

#[derive(Debug, Clone, PartialEq)]
pub struct PolyShape {
    pub polygon: Polygon,
}

impl MeasureLength for PolyShape {
    fn length(&self) -> f64 {
        let mut length = 0.0;

        for line in self.polygon.exterior().lines() {
            length += line.length::<Euclidean>();
        }

        length
    }
}

impl AccessShape for PolyShape {
    fn center(&self) -> Point {
        self.polygon.centroid().unwrap()
    }

    fn contains_point(&self, p: Point) -> bool {
        self.polygon.contains(&p)
    }

    fn bbox_without_margin(&self) -> AABB<[f64; 2]> {
        AABB::from_points(
            self.polygon
                .exterior()
                .0
                .iter()
                .map(|coord| [coord.x, coord.y])
                .collect::<Vec<_>>()
                .iter(),
        )
    }
}
