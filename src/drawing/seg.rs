use enum_dispatch::enum_dispatch;

use crate::{
    drawing::{
        graph::{GetLayer, GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight, Retag},
        primitive::{GenericPrimitive, Primitive},
        rules::RulesTrait,
        Drawing,
    },
    geometry::{GetWidth, SegWeightTrait},
    graph::{GenericIndex, GetPetgraphIndex},
};

use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    LoneLoose(LoneLooseSegIndex),
    SeqLoose(SeqLooseSegIndex),
}

impl From<SegIndex> for PrimitiveIndex {
    fn from(seg: SegIndex) -> Self {
        match seg {
            SegIndex::Fixed(seg) => PrimitiveIndex::FixedSeg(seg),
            SegIndex::LoneLoose(seg) => PrimitiveIndex::LoneLooseSeg(seg),
            SegIndex::SeqLoose(seg) => PrimitiveIndex::SeqLooseSeg(seg),
        }
    }
}

impl TryFrom<PrimitiveIndex> for SegIndex {
    type Error = (); // TODO.

    fn try_from(index: PrimitiveIndex) -> Result<SegIndex, ()> {
        match index {
            PrimitiveIndex::FixedSeg(index) => Ok(SegIndex::Fixed(index)),
            PrimitiveIndex::LoneLooseSeg(index) => Ok(SegIndex::LoneLoose(index)),
            PrimitiveIndex::SeqLooseSeg(index) => Ok(SegIndex::SeqLoose(index)),
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

impl From<SegWeight> for PrimitiveWeight {
    fn from(seg: SegWeight) -> Self {
        match seg {
            SegWeight::Fixed(weight) => PrimitiveWeight::FixedSeg(weight),
            SegWeight::LoneLoose(weight) => PrimitiveWeight::LoneLooseSeg(weight),
            SegWeight::SeqLoose(weight) => PrimitiveWeight::SeqLooseSeg(weight),
        }
    }
}

impl TryFrom<PrimitiveWeight> for SegWeight {
    type Error = (); // TODO.

    fn try_from(weight: PrimitiveWeight) -> Result<SegWeight, ()> {
        match weight {
            PrimitiveWeight::FixedSeg(weight) => Ok(SegWeight::Fixed(weight)),
            PrimitiveWeight::LoneLooseSeg(weight) => Ok(SegWeight::LoneLoose(weight)),
            PrimitiveWeight::SeqLooseSeg(weight) => Ok(SegWeight::SeqLoose(weight)),
            _ => Err(()),
        }
    }
}

impl SegWeightTrait<PrimitiveWeight> for SegWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub width: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_fixed_weight!(FixedSegWeight, FixedSeg, FixedSegIndex);
impl SegWeightTrait<PrimitiveWeight> for FixedSegWeight {}

impl GetWidth for FixedSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoneLooseSegWeight {
    pub width: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_loose_weight!(LoneLooseSegWeight, LoneLooseSeg, LoneLooseSegIndex);
impl SegWeightTrait<PrimitiveWeight> for LoneLooseSegWeight {}

impl GetWidth for LoneLooseSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeqLooseSegWeight {
    pub width: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl_loose_weight!(SeqLooseSegWeight, SeqLooseSeg, SeqLooseSegIndex);
impl SegWeightTrait<PrimitiveWeight> for SeqLooseSegWeight {}

impl GetWidth for SeqLooseSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}
