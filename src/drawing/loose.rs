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
    graph::GetPetgraphIndex,
};

use super::rules::AccessRules;

#[enum_dispatch]
pub trait GetPrevNextLoose {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex>;

    fn prev_loose(&self, maybe_next: Option<LooseIndex>) -> Option<LooseIndex> {
        if maybe_next.is_some() {
            self.next_loose(maybe_next)
        } else {
            self.next_loose(self.next_loose(None))
        }
    }
}

#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
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

#[enum_dispatch(GetPrevNextLoose, GetDrawing, GetPetgraphIndex)]
pub enum Loose<'a, CW: Copy, R: AccessRules> {
    Dot(LooseDot<'a, CW, R>),
    LoneSeg(LoneLooseSeg<'a, CW, R>),
    SeqSeg(SeqLooseSeg<'a, CW, R>),
    Bend(LooseBend<'a, CW, R>),
}

impl<'a, CW: Copy, R: AccessRules> Loose<'a, CW, R> {
    pub fn new(index: LooseIndex, drawing: &'a Drawing<CW, R>) -> Self {
        match index {
            LooseIndex::Dot(dot) => drawing.primitive(dot).into(),
            LooseIndex::LoneSeg(seg) => drawing.primitive(seg).into(),
            LooseIndex::SeqSeg(seg) => drawing.primitive(seg).into(),
            LooseIndex::Bend(bend) => drawing.primitive(bend).into(),
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetPrevNextLoose for LooseDot<'a, CW, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let bend = self.bend();
        let Some(prev) = maybe_prev else {
            unreachable!();
        };

        if bend.petgraph_index() != prev.petgraph_index() {
            Some(bend.into())
        } else {
            self.seg().map(Into::into)
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetPrevNextLoose for LoneLooseSeg<'a, CW, R> {
    fn next_loose(&self, _maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        None
    }
}

impl<'a, CW: Copy, R: AccessRules> GetPrevNextLoose for SeqLooseSeg<'a, CW, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let joints = self.joints();
        let Some(prev) = maybe_prev else {
            return Some(joints.1.into());
        };

        if joints.0.petgraph_index() != prev.petgraph_index() {
            match joints.0 {
                DotIndex::Fixed(..) => None,
                DotIndex::Loose(dot) => Some(dot.into()),
            }
        } else {
            Some(joints.1.into())
        }
    }
}

impl<'a, CW: Copy, R: AccessRules> GetPrevNextLoose for LooseBend<'a, CW, R> {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        let joints = self.joints();
        let Some(prev) = maybe_prev else {
            unreachable!();
        };

        if joints.0.petgraph_index() != prev.petgraph_index() {
            Some(joints.0.into())
        } else {
            Some(joints.1.into())
        }
    }
}
