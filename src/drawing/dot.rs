use enum_dispatch::enum_dispatch;
use geo::Point;

use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    geometry::{DotWeightTrait, GetPos, GetWidth, SetPos},
    graph::{GenericIndex, GetPetgraphIndex},
    math::Circle,
};

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotIndex {
    Fixed(FixedDotIndex),
    Loose(LooseDotIndex),
}

impl From<DotIndex> for PrimitiveIndex {
    fn from(dot: DotIndex) -> Self {
        match dot {
            DotIndex::Fixed(index) => PrimitiveIndex::FixedDot(index),
            DotIndex::Loose(index) => PrimitiveIndex::LooseDot(index),
        }
    }
}

impl TryFrom<PrimitiveIndex> for DotIndex {
    type Error = (); // TODO.

    fn try_from(index: PrimitiveIndex) -> Result<DotIndex, ()> {
        match index {
            PrimitiveIndex::FixedDot(index) => Ok(DotIndex::Fixed(index)),
            PrimitiveIndex::LooseDot(index) => Ok(DotIndex::Loose(index)),
            _ => Err(()),
        }
    }
}

#[enum_dispatch(GetPos, SetPos, GetWidth, GetLayer)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotWeight {
    Fixed(FixedDotWeight),
    Loose(LooseDotWeight),
}

impl From<DotWeight> for PrimitiveWeight {
    fn from(dot: DotWeight) -> Self {
        match dot {
            DotWeight::Fixed(weight) => PrimitiveWeight::FixedDot(weight),
            DotWeight::Loose(weight) => PrimitiveWeight::LooseDot(weight),
        }
    }
}

impl TryFrom<PrimitiveWeight> for DotWeight {
    type Error = (); // TODO.

    fn try_from(weight: PrimitiveWeight) -> Result<DotWeight, ()> {
        match weight {
            PrimitiveWeight::FixedDot(weight) => Ok(DotWeight::Fixed(weight)),
            PrimitiveWeight::LooseDot(weight) => Ok(DotWeight::Loose(weight)),
            _ => Err(()),
        }
    }
}

impl DotWeightTrait<PrimitiveWeight> for DotWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub circle: Circle,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_fixed_weight!(FixedDotWeight, FixedDot, FixedDotIndex);
impl DotWeightTrait<PrimitiveWeight> for FixedDotWeight {}

impl GetPos for FixedDotWeight {
    fn pos(&self) -> Point {
        self.circle.pos
    }
}

impl SetPos for FixedDotWeight {
    fn set_pos(&mut self, pos: Point) {
        self.circle.pos = pos
    }
}

impl GetWidth for FixedDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight {
    pub circle: Circle,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_loose_weight!(LooseDotWeight, LooseDot, LooseDotIndex);
impl DotWeightTrait<PrimitiveWeight> for LooseDotWeight {}

impl GetPos for LooseDotWeight {
    fn pos(&self) -> Point {
        self.circle.pos
    }
}

impl SetPos for LooseDotWeight {
    fn set_pos(&mut self, pos: Point) {
        self.circle.pos = pos
    }
}

impl GetWidth for LooseDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}
