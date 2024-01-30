use enum_dispatch::enum_dispatch;
use geo::Point;

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::{GenericIndex, GetNodeIndex},
    layout::Layout,
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

use super::geometry::{
    DotWeightTrait, GeometryIndex, GeometryWeight, GetBandIndex, GetComponentIndex,
    GetComponentIndexMut, GetPos, GetWidth, MakePrimitive, Retag,
};
use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotIndex {
    Fixed(FixedDotIndex),
    Loose(LooseDotIndex),
}

impl From<DotIndex> for GeometryIndex {
    fn from(dot: DotIndex) -> Self {
        match dot {
            DotIndex::Fixed(index) => GeometryIndex::FixedDot(index),
            DotIndex::Loose(index) => GeometryIndex::LooseDot(index),
        }
    }
}

impl TryFrom<GeometryIndex> for DotIndex {
    type Error = (); // TODO.

    fn try_from(index: GeometryIndex) -> Result<DotIndex, ()> {
        match index {
            GeometryIndex::FixedDot(index) => Ok(DotIndex::Fixed(index)),
            GeometryIndex::LooseDot(index) => Ok(DotIndex::Loose(index)),
            _ => Err(()),
        }
    }
}

#[enum_dispatch(GetPos, GetWidth)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotWeight {
    Fixed(FixedDotWeight),
    Loose(LooseDotWeight),
}

impl From<DotWeight> for GeometryWeight {
    fn from(dot: DotWeight) -> Self {
        match dot {
            DotWeight::Fixed(weight) => GeometryWeight::FixedDot(weight),
            DotWeight::Loose(weight) => GeometryWeight::LooseDot(weight),
        }
    }
}

impl TryFrom<GeometryWeight> for DotWeight {
    type Error = (); // TODO.

    fn try_from(weight: GeometryWeight) -> Result<DotWeight, ()> {
        match weight {
            GeometryWeight::FixedDot(weight) => Ok(DotWeight::Fixed(weight)),
            GeometryWeight::LooseDot(weight) => Ok(DotWeight::Loose(weight)),
            _ => Err(()),
        }
    }
}

impl DotWeightTrait<GeometryWeight> for DotWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub component: ComponentIndex,
    pub circle: Circle,
}

impl_fixed_weight!(FixedDotWeight, FixedDot, FixedDotIndex);
impl DotWeightTrait<GeometryWeight> for FixedDotWeight {}

impl GetPos for FixedDotWeight {
    fn pos(&self) -> Point {
        self.circle.pos
    }
}

impl GetWidth for FixedDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight {
    pub band: BandIndex,
    pub circle: Circle,
}

impl_loose_weight!(LooseDotWeight, LooseDot, LooseDotIndex);
impl DotWeightTrait<GeometryWeight> for LooseDotWeight {}

impl GetPos for LooseDotWeight {
    fn pos(&self) -> Point {
        self.circle.pos
    }
}

impl GetWidth for LooseDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}
