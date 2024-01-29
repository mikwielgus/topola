use enum_dispatch::enum_dispatch;

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::GenericIndex,
    layout::{GetNodeIndex, Layout},
    primitive::{GenericPrimitive, Primitive},
};

use super::geometry::{
    BendWeightTrait, GeometryIndex, GeometryWeight, GetBandIndex, GetComponentIndex,
    GetComponentIndexMut, GetOffset, GetWidth, MakePrimitive, Retag,
};

use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendIndex {
    Fixed(FixedBendIndex),
    Loose(LooseBendIndex),
}

impl From<BendIndex> for GeometryIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => GeometryIndex::FixedBend(bend),
            BendIndex::Loose(bend) => GeometryIndex::LooseBend(bend),
        }
    }
}

impl TryFrom<GeometryIndex> for BendIndex {
    type Error = (); // TODO.

    fn try_from(index: GeometryIndex) -> Result<BendIndex, ()> {
        match index {
            GeometryIndex::FixedBend(index) => Ok(BendIndex::Fixed(index)),
            GeometryIndex::LooseBend(index) => Ok(BendIndex::Loose(index)),
            _ => Err(()),
        }
    }
}

#[enum_dispatch(GetOffset, GetWidth)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendWeight {
    Fixed(FixedBendWeight),
    Loose(LooseBendWeight),
}

impl From<BendWeight> for GeometryWeight {
    fn from(bend: BendWeight) -> Self {
        match bend {
            BendWeight::Fixed(weight) => GeometryWeight::FixedBend(weight),
            BendWeight::Loose(weight) => GeometryWeight::LooseBend(weight),
        }
    }
}

impl TryFrom<GeometryWeight> for BendWeight {
    type Error = (); // TODO.

    fn try_from(weight: GeometryWeight) -> Result<BendWeight, ()> {
        match weight {
            GeometryWeight::FixedBend(weight) => Ok(BendWeight::Fixed(weight)),
            GeometryWeight::LooseBend(weight) => Ok(BendWeight::Loose(weight)),
            _ => Err(()),
        }
    }
}

impl BendWeightTrait<GeometryWeight> for BendWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub component: ComponentIndex,
    pub width: f64,
    pub offset: f64,
    pub cw: bool,
}

impl_fixed_weight!(FixedBendWeight, FixedBend, FixedBendIndex);
impl BendWeightTrait<GeometryWeight> for FixedBendWeight {}

impl GetOffset for FixedBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl GetWidth for FixedBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub band: BandIndex,
    pub width: f64,
    pub offset: f64,
    pub cw: bool,
}

impl GetOffset for LooseBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl GetWidth for LooseBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

impl_loose_weight!(LooseBendWeight, LooseBend, LooseBendIndex);
impl BendWeightTrait<GeometryWeight> for LooseBendWeight {}
