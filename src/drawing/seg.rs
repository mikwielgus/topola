// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;

use crate::{
    drawing::{
        graph::{GetMaybeNet, MakePrimitive, PrimitiveIndex, PrimitiveWeight},
        loose::LooseIndex,
        primitive::{GenericPrimitive, Primitive},
        rules::AccessRules,
        Drawing,
    },
    geometry::{GetLayer, GetWidth},
    graph::{GenericIndex, GetPetgraphIndex},
};

use petgraph::stable_graph::NodeIndex;

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    LoneLoose(LoneLooseSegIndex),
    SeqLoose(SeqLooseSegIndex),
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum LooseSegIndex {
    Lone(LoneLooseSegIndex),
    Seq(SeqLooseSegIndex),
}

impl From<LooseSegIndex> for SegIndex {
    fn from(seg: LooseSegIndex) -> Self {
        match seg {
            LooseSegIndex::Lone(seg) => SegIndex::LoneLoose(seg),
            LooseSegIndex::Seq(seg) => SegIndex::SeqLoose(seg),
        }
    }
}

impl From<LooseSegIndex> for LooseIndex {
    fn from(seg: LooseSegIndex) -> Self {
        match seg {
            LooseSegIndex::Lone(seg) => LooseIndex::LoneSeg(seg),
            LooseSegIndex::Seq(seg) => LooseIndex::SeqSeg(seg),
        }
    }
}

impl TryFrom<SegIndex> for LooseSegIndex {
    type Error = (); // TODO.

    fn try_from(index: SegIndex) -> Result<LooseSegIndex, ()> {
        Ok(match index {
            SegIndex::LoneLoose(index) => LooseSegIndex::Lone(index),
            SegIndex::SeqLoose(index) => LooseSegIndex::Seq(index),
            _ => return Err(()),
        })
    }
}

impl TryFrom<LooseIndex> for LooseSegIndex {
    type Error = (); // TODO.

    fn try_from(index: LooseIndex) -> Result<LooseSegIndex, ()> {
        Ok(match index {
            LooseIndex::LoneSeg(index) => LooseSegIndex::Lone(index),
            LooseIndex::SeqSeg(index) => LooseSegIndex::Seq(index),
            _ => return Err(()),
        })
    }
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
        Ok(match index {
            PrimitiveIndex::FixedSeg(index) => SegIndex::Fixed(index),
            PrimitiveIndex::LoneLooseSeg(index) => SegIndex::LoneLoose(index),
            PrimitiveIndex::SeqLooseSeg(index) => SegIndex::SeqLoose(index),
            _ => return Err(()),
        })
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight(pub GeneralSegWeight);
impl_weight_forward!(FixedSegWeight, FixedSeg, FixedSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoneLooseSegWeight(pub GeneralSegWeight);
impl_weight_forward!(LoneLooseSegWeight, LoneLooseSeg, LoneLooseSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeqLooseSegWeight(pub GeneralSegWeight);
impl_weight_forward!(SeqLooseSegWeight, SeqLooseSeg, SeqLooseSegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneralSegWeight {
    pub width: f64,
    pub layer: usize,
    pub maybe_net: Option<usize>,
}

impl GetLayer for GeneralSegWeight {
    fn layer(&self) -> usize {
        self.layer
    }
}

impl GetMaybeNet for GeneralSegWeight {
    fn maybe_net(&self) -> Option<usize> {
        self.maybe_net
    }
}

impl GetWidth for GeneralSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}
