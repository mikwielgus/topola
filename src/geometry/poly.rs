use geo::{Centroid, Contains, Point, Polygon};

use crate::geometry::shape::ShapeTrait;

#[derive(Debug, Clone, PartialEq)]
pub struct PolyShape {
    pub polygon: Polygon,
}

impl ShapeTrait for PolyShape {
    fn center(&self) -> Point {
        self.polygon.centroid().unwrap()
    }

    fn contains_point(&self, p: Point) -> bool {
        self.polygon.contains(&p)
    }
}
