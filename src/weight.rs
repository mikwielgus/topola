use enum_as_inner::EnumAsInner;
use crate::{math::Circle, mesh::DotIndex};

#[derive(Clone, Copy, PartialEq)]
pub struct DotWeight {
    pub net: i32,
    pub circle: Circle,
}

#[derive(Clone, Copy, PartialEq)]
pub struct BendWeight {
    pub net: i32,
    pub around: DotIndex,
    pub cw: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub struct SegWeight {
    pub net: i32,
    pub width: f64,
}

#[derive(EnumAsInner, Clone, Copy, PartialEq)]
pub enum Weight {
    Dot(DotWeight),
    Seg(SegWeight),
    Bend(BendWeight),
}
