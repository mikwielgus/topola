use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::Drawing,
    drawing::{
        bend::LooseBendIndex,
        dot::{DotIndex, LooseDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{GetJoints, LoneLooseSeg, LooseBend, LooseDot, Primitive, SeqLooseSeg},
        seg::{LoneLooseSegIndex, SeqLooseSegIndex},
    },
    graph::GetNodeIndex,
};

use super::rules::RulesTrait;

#[enum_dispatch]
pub trait GetNextLoose {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex>;
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LooseIndex {
    Dot(LooseDotIndex),
    LoneSeg(LoneLooseSegIndex),
    SeqSeg(SeqLooseSegIndex),
    Bend(LooseBendIndex),
}

impl From<LooseIndex> for PrimitiveIndex {
    fn from(loose: LooseIndex) -> Self {
        match loose {
            LooseIndex::Dot(dot) => PrimitiveIndex::LooseDot(dot),
            LooseIndex::LoneSeg(seg) => PrimitiveIndex::LoneLooseSeg(seg),
            LooseIndex::SeqSeg(seg) => PrimitiveIndex::SeqLooseSeg(seg),
            LooseIndex::Bend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetNextLoose, GetLayout, GetNodeIndex)]
pub enum Loose<'a, R: RulesTrait> {
    Dot(LooseDot<'a, R>),
    LoneSeg(LoneLooseSeg<'a, R>),
    SeqSeg(SeqLooseSeg<'a, R>),
    Bend(LooseBend<'a, R>),
}

impl<'a, R: RulesTrait> Loose<'a, R> {
    pub fn new(index: LooseIndex, drawing: &'a Drawing<R>) -> Self {
        match index {
            LooseIndex::Dot(dot) => drawing.primitive(dot).into(),
            LooseIndex::LoneSeg(seg) => drawing.primitive(seg).into(),
            LooseIndex::SeqSeg(seg) => drawing.primitive(seg).into(),
            LooseIndex::Bend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, R: RulesTrait> GetNextLoose for LooseDot<'a, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let bend = self.bend();
        let Some(prev) = maybe_prev else {
            unreachable!();
        };

        if bend.node_index() != prev.node_index() {
            Some(bend.into())
        } else {
            self.seg().map(Into::into)
        }
    }
}

impl<'a, R: RulesTrait> GetNextLoose for LoneLooseSeg<'a, R> {
    fn next_loose(&self, _maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        None
    }
}

impl<'a, R: RulesTrait> GetNextLoose for SeqLooseSeg<'a, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let ends = self.joints();
        let Some(prev) = maybe_prev else {
            return Some(ends.1.into());
        };

        if ends.0.node_index() != prev.node_index() {
            match ends.0 {
                DotIndex::Fixed(..) => None,
                DotIndex::Loose(dot) => Some(dot.into()),
            }
        } else {
            Some(ends.1.into())
        }
    }
}

impl<'a, R: RulesTrait> GetNextLoose for LooseBend<'a, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let ends = self.joints();
        let Some(prev) = maybe_prev else {
            unreachable!();
        };

        if ends.0.node_index() != prev.node_index() {
            Some(ends.0.into())
        } else {
            Some(ends.1.into())
        }
    }
}
