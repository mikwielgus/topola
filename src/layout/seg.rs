use enum_dispatch::enum_dispatch;

use crate::{
    geometry::{GetWidth, SegWeightTrait},
    graph::{GenericIndex, GetNodeIndex},
    layout::{
        graph::{GeometryIndex, GeometryWeight, GetLayer, GetNet, MakePrimitive, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Layout,
    },
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

impl TryFrom<GeometryIndex> for SegIndex {
    type Error = (); // TODO.

    fn try_from(index: GeometryIndex) -> Result<SegIndex, ()> {
        match index {
            GeometryIndex::FixedSeg(index) => Ok(SegIndex::Fixed(index)),
            GeometryIndex::LoneLooseSeg(index) => Ok(SegIndex::LoneLoose(index)),
            GeometryIndex::SeqLooseSeg(index) => Ok(SegIndex::SeqLoose(index)),
            _ => Err(()),
        }
    }
}

#[enum_dispatch(GetWidth, GetLayer)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegWeight {
    Fixed(FixedSegWeight),
    LoneLoose(LoneLooseSegWeight),
    SeqLoose(SeqLooseSegWeight),
}

impl From<SegWeight> for GeometryWeight {
    fn from(seg: SegWeight) -> Self {
        match seg {
            SegWeight::Fixed(weight) => GeometryWeight::FixedSeg(weight),
            SegWeight::LoneLoose(weight) => GeometryWeight::LoneLooseSeg(weight),
            SegWeight::SeqLoose(weight) => GeometryWeight::SeqLooseSeg(weight),
        }
    }
}

impl TryFrom<GeometryWeight> for SegWeight {
    type Error = (); // TODO.

    fn try_from(weight: GeometryWeight) -> Result<SegWeight, ()> {
        match weight {
            GeometryWeight::FixedSeg(weight) => Ok(SegWeight::Fixed(weight)),
            GeometryWeight::LoneLooseSeg(weight) => Ok(SegWeight::LoneLoose(weight)),
            GeometryWeight::SeqLooseSeg(weight) => Ok(SegWeight::SeqLoose(weight)),
            _ => Err(()),
        }
    }
}

impl SegWeightTrait<GeometryWeight> for SegWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub width: f64,
    pub layer: u64,
    pub net: i64,
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
    pub width: f64,
    pub layer: u64,
    pub net: i64,
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
    pub width: f64,
    pub layer: u64,
    pub net: i64,
}

impl_loose_weight!(SeqLooseSegWeight, SeqLooseSeg, SeqLooseSegIndex);
impl SegWeightTrait<GeometryWeight> for SeqLooseSegWeight {}

impl GetWidth for SeqLooseSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}
