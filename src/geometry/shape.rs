use enum_dispatch::enum_dispatch;
use geo::Point;

use crate::geometry::primitive::PrimitiveShape;

#[enum_dispatch]
pub trait ShapeTrait {
    fn contains_point(&self, p: Point) -> bool;
}
