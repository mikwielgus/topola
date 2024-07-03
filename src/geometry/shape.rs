use enum_dispatch::enum_dispatch;
use geo::Point;

use crate::geometry::{
    poly::PolyShape,
    primitive::{BendShape, DotShape, PrimitiveShape, SegShape},
};

#[enum_dispatch]
pub trait MeasureLength {
    fn length(&self) -> f64;
}

#[enum_dispatch]
pub trait AccessShape: MeasureLength {
    fn center(&self) -> Point;
    fn contains_point(&self, p: Point) -> bool;
}

#[enum_dispatch(MeasureLength, AccessShape)]
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Dot(DotShape),
    Seg(SegShape),
    Bend(BendShape),
    Poly(PolyShape),
}

impl From<PrimitiveShape> for Shape {
    fn from(primitive: PrimitiveShape) -> Self {
        match primitive {
            PrimitiveShape::Dot(dot) => Shape::Dot(dot),
            PrimitiveShape::Seg(seg) => Shape::Seg(seg),
            PrimitiveShape::Bend(bend) => Shape::Bend(bend),
        }
    }
}
