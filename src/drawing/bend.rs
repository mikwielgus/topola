// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;

use crate::{
    drawing::{
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight},
        primitive::{GenericPrimitive, Primitive},
        rules::AccessRules,
        Drawing,
    },
    geometry::{GetLayer, GetOffset, GetWidth, SetOffset},
    graph::{GenericIndex, GetPetgraphIndex},
};

use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BendIndex {
    Fixed(FixedBendIndex),
    Loose(LooseBendIndex),
}

impl From<BendIndex> for PrimitiveIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => PrimitiveIndex::FixedBend(bend),
            BendIndex::Loose(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl TryFrom<PrimitiveIndex> for BendIndex {
    type Error = (); // TODO.

    fn try_from(index: PrimitiveIndex) -> Result<BendIndex, ()> {
        match index {
            PrimitiveIndex::FixedBend(index) => Ok(BendIndex::Fixed(index)),
            PrimitiveIndex::LooseBend(index) => Ok(BendIndex::Loose(index)),
            _ => Err(()),
        }
    }
}

#[enum_dispatch(GetOffset, SetOffset, GetWidth, GetLayer)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendWeight {
    Fixed(FixedBendWeight),
    Loose(LooseBendWeight),
}

impl From<BendWeight> for PrimitiveWeight {
    fn from(bend: BendWeight) -> Self {
        match bend {
            BendWeight::Fixed(weight) => PrimitiveWeight::FixedBend(weight),
            BendWeight::Loose(weight) => PrimitiveWeight::LooseBend(weight),
        }
    }
}

impl TryFrom<PrimitiveWeight> for BendWeight {
    type Error = (); // TODO.

    fn try_from(weight: PrimitiveWeight) -> Result<BendWeight, ()> {
        match weight {
            PrimitiveWeight::FixedBend(weight) => Ok(BendWeight::Fixed(weight)),
            PrimitiveWeight::LooseBend(weight) => Ok(BendWeight::Loose(weight)),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight(pub GeneralBendWeight);
impl_weight_forward!(FixedBendWeight, FixedBend, FixedBendIndex);

impl GetOffset for FixedBendWeight {
    fn offset(&self) -> f64 {
        self.0.offset()
    }
}

impl SetOffset for FixedBendWeight {
    fn set_offset(&mut self, offset: f64) {
        self.0.set_offset(offset);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight(pub GeneralBendWeight);
impl_weight_forward!(LooseBendWeight, LooseBend, LooseBendIndex);

impl GetOffset for LooseBendWeight {
    fn offset(&self) -> f64 {
        self.0.offset()
    }
}

impl SetOffset for LooseBendWeight {
    fn set_offset(&mut self, offset: f64) {
        self.0.set_offset(offset);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneralBendWeight {
    pub width: f64,
    pub offset: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for GeneralBendWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for GeneralBendWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl GetOffset for GeneralBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl SetOffset for GeneralBendWeight {
    fn set_offset(&mut self, offset: f64) {
        self.offset = offset
    }
}

impl GetWidth for GeneralBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}
