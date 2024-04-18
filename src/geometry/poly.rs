use enum_dispatch::enum_dispatch;
use geo::{Contains, Point, Polygon};

use crate::geometry::shape::ShapeTrait;

#[derive(Debug, Clone, PartialEq)]
pub struct PolyShape {
    pub polygon: Polygon,
}

impl ShapeTrait for PolyShape {
    fn contains_point(&self, p: Point) -> bool {
        self.polygon.contains(&p)
    }
}
