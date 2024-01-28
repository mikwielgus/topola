use enum_dispatch::enum_dispatch;

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::GenericIndex,
    layout::{GetNodeIndex, Layout},
    primitive::{GenericPrimitive, Primitive},
};

use super::geometry::{
    GeometryIndex, GeometryWeight, GetBandIndex, GetComponentIndex, GetComponentIndexMut, GetWidth,
    MakePrimitive, Retag, SegWeightTrait,
};
use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    LoneLoose(LoneLooseSegIndex),
    SeqLoose(SeqLooseSegIndex),
}

impl From<SegIndex> for GeometryIndex {
    fn from(seg: SegIndex) -> Self {
        match seg {
            SegIndex::Fixed(seg) => GeometryIndex::FixedSeg(seg),
            SegIndex::LoneLoose(seg) => GeometryIndex::LoneLooseSeg(seg),
            SegIndex::SeqLoose(seg) => GeometryIndex::SeqLooseSeg(seg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub component: ComponentIndex,
    pub width: f64,
}

impl_fixed_weight!(FixedSegWeight, FixedSeg, FixedSegIndex);
impl SegWeightTrait<GeometryWeight> for FixedSegWeight {}

impl GetWidth for FixedSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoneLooseSegWeight {
    pub band: BandIndex,
    pub width: f64,
}

impl_loose_weight!(LoneLooseSegWeight, LoneLooseSeg, LoneLooseSegIndex);
impl SegWeightTrait<GeometryWeight> for LoneLooseSegWeight {}

impl GetWidth for LoneLooseSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeqLooseSegWeight {
    pub band: BandIndex,
    pub width: f64,
}

impl_loose_weight!(SeqLooseSegWeight, SeqLooseSeg, SeqLooseSegIndex);
impl SegWeightTrait<GeometryWeight> for SeqLooseSegWeight {}

impl GetWidth for SeqLooseSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}
