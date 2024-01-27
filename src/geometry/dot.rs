use enum_dispatch::enum_dispatch;

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::GenericIndex,
    layout::Layout,
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

use super::geometry::{
    GeometryIndex, GeometryWeight, GetBandIndex, GetComponentIndex, GetComponentIndexMut, GetWidth,
    MakePrimitive, Retag,
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
            DotIndex::Fixed(fixed) => GeometryIndex::FixedDot(fixed),
            DotIndex::Loose(loose) => GeometryIndex::LooseDot(loose),
        }
    }
}

pub trait DotWeight: GetWidth + Into<GeometryWeight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub component: ComponentIndex,
    pub circle: Circle,
}

impl_fixed_weight!(FixedDotWeight, FixedDot, FixedDotIndex);
impl DotWeight for FixedDotWeight {}

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
impl DotWeight for LooseDotWeight {}

impl GetWidth for LooseDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}
