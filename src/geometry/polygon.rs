// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::algorithm::{
    line_measures::{Euclidean, Length},
    Intersects,
};
use geo::{Centroid, Contains, Point, Polygon};
use rstar::AABB;

use crate::geometry::shape::{AccessShape, MeasureLength};

impl MeasureLength for Polygon {
    fn length(&self) -> f64 {
        let mut length = 0.0;

        for line in self.exterior().lines() {
            length += line.length::<Euclidean>();
        }

        length
    }
}

impl AccessShape for Polygon {
    fn center(&self) -> Point {
        self.centroid().unwrap()
    }

    fn contains_point(&self, p: Point) -> bool {
        self.contains(&p)
    }

    fn bbox_without_margin(&self) -> AABB<[f64; 2]> {
        AABB::from_points(
            self.exterior()
                .0
                .iter()
                .map(|coord| [coord.x, coord.y])
                .collect::<Vec<_>>()
                .iter(),
        )
    }

    fn intersects_with_bbox(&self, bbox: &AABB<[f64; 2]>) -> bool {
        geo::Rect::new(bbox.lower(), bbox.upper()).intersects(self)
    }
}
