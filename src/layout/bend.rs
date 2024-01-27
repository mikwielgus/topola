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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub component: ComponentIndex,
    pub width: f64,
    pub cw: bool,
}

impl_fixed_weight!(FixedBendWeight, FixedBend, FixedBendIndex);
impl BendWeightTrait<GeometryWeight> for FixedBendWeight {}

impl GetWidth for FixedBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub band: BandIndex,
    pub offset: f64,
    pub cw: bool,
}

impl GetOffset for LooseBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl_loose_weight!(LooseBendWeight, LooseBend, LooseBendIndex);
impl BendWeightTrait<GeometryWeight> for LooseBendWeight {}
