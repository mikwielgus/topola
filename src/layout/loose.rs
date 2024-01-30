use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    graph::GetNodeIndex,
    layout::Layout,
    layout::{
        bend::LooseBendIndex,
        dot::{DotIndex, LooseDotIndex},
        geometry::{GeometryIndex, MakePrimitive},
        seg::{LoneLooseSegIndex, SeqLooseSegIndex},
    },
    primitive::{GetJoints, LoneLooseSeg, LooseBend, LooseDot, Primitive, SeqLooseSeg},
};

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

impl From<LooseIndex> for GeometryIndex {
    fn from(loose: LooseIndex) -> Self {
        match loose {
            LooseIndex::Dot(dot) => GeometryIndex::LooseDot(dot),
            LooseIndex::LoneSeg(seg) => GeometryIndex::LoneLooseSeg(seg),
            LooseIndex::SeqSeg(seg) => GeometryIndex::SeqLooseSeg(seg),
            LooseIndex::Bend(bend) => GeometryIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetNextLoose, GetLayout, GetNodeIndex)]
pub enum Loose<'a> {
    Dot(LooseDot<'a>),
    LoneSeg(LoneLooseSeg<'a>),
    SeqSeg(SeqLooseSeg<'a>),
    Bend(LooseBend<'a>),
}

impl<'a> Loose<'a> {
    pub fn new(index: LooseIndex, layout: &'a Layout) -> Self {
        match index {
            LooseIndex::Dot(dot) => layout.primitive(dot).into(),
            LooseIndex::LoneSeg(seg) => layout.primitive(seg).into(),
            LooseIndex::SeqSeg(seg) => layout.primitive(seg).into(),
            LooseIndex::Bend(bend) => layout.primitive(bend).into(),
        }
    }
}

impl<'a> GetNextLoose for LooseDot<'a> {
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

impl<'a> GetNextLoose for LoneLooseSeg<'a> {
    fn next_loose(&self, _maybe_prev: Option<LooseIndex>) -> Option<LooseIndex> {
        None
    }
}

impl<'a> GetNextLoose for SeqLooseSeg<'a> {
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

impl<'a> GetNextLoose for LooseBend<'a> {
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
