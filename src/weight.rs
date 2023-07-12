use enum_as_inner::EnumAsInner;
use crate::{math::Circle, mesh::{Index, DotIndex}};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotWeight {
    pub net: i32,
    pub circle: Circle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendWeight {
    pub net: i32,
    pub around: Index,
    pub center: DotIndex,
    pub cw: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegWeight {
    pub net: i32,
    pub width: f64,
}

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Weight {
    Dot(DotWeight),
    Seg(SegWeight),
    Bend(BendWeight),
}
