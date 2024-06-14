use enum_dispatch::enum_dispatch;

use crate::{
    drawing::{
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    geometry::{BendWeightTrait, GetOffset, GetWidth, SetOffset},
    graph::{GenericIndex, GetNodeIndex},
};

use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
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

impl BendWeightTrait<PrimitiveWeight> for BendWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub width: f64,
    pub offset: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_fixed_weight!(FixedBendWeight, FixedBend, FixedBendIndex);
impl BendWeightTrait<PrimitiveWeight> for FixedBendWeight {}

impl GetOffset for FixedBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl SetOffset for FixedBendWeight {
    fn set_offset(&mut self, offset: f64) {
        self.offset = offset
    }
}

impl GetWidth for FixedBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub width: f64,
    pub offset: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetOffset for LooseBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl SetOffset for LooseBendWeight {
    fn set_offset(&mut self, offset: f64) {
        self.offset = offset
    }
}

impl GetWidth for LooseBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

impl_loose_weight!(LooseBendWeight, LooseBend, LooseBendIndex);
impl BendWeightTrait<PrimitiveWeight> for LooseBendWeight {}
