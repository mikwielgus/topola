// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use geo::Point;
use rstar::AABB;

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
    fn bbox_without_margin(&self) -> AABB<[f64; 2]>;

    fn bbox(&self, margin: f64) -> AABB<[f64; 2]> {
        let aabb = self.bbox_without_margin();
        AABB::<[f64; 2]>::from_corners(
            [aabb.lower()[0] - margin, aabb.lower()[1] - margin],
            [aabb.upper()[0] + margin, aabb.upper()[1] + margin],
        )
    }
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
