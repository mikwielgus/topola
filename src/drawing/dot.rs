// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use geo::Point;

use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::{
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight},
        primitive::{GenericPrimitive, Primitive},
        rules::AccessRules,
        Drawing,
    },
    geometry::{GetSetPos, GetWidth},
    graph::{GenericIndex, GetPetgraphIndex},
    math::Circle,
};

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[enum_dispatch(GetSetPos, GetWidth, GetLayer)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight(pub GeneralDotWeight);
impl_weight_forward!(FixedDotWeight, FixedDot, FixedDotIndex);

impl GetSetPos for FixedDotWeight {
    fn pos(&self) -> Point {
        self.0.pos()
    }
    fn set_pos(&mut self, pos: Point) {
        self.0.set_pos(pos);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight(pub GeneralDotWeight);
impl_weight_forward!(LooseDotWeight, LooseDot, LooseDotIndex);

impl GetSetPos for LooseDotWeight {
    fn pos(&self) -> Point {
        self.0.pos()
    }
    fn set_pos(&mut self, pos: Point) {
        self.0.set_pos(pos);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneralDotWeight {
    pub circle: Circle,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for GeneralDotWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for GeneralDotWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl GetSetPos for GeneralDotWeight {
    fn pos(&self) -> Point {
        self.circle.pos
    }

    fn set_pos(&mut self, pos: Point) {
        self.circle.pos = pos;
    }
}

impl GetWidth for GeneralDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}
