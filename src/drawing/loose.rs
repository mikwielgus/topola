// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    drawing::Drawing,
    drawing::{
        bend::LooseBendIndex,
        dot::{DotIndex, LooseDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::{GetJoints, LoneLooseSeg, LooseBend, LooseDot, Primitive, SeqLooseSeg},
        rules::AccessRules,
        seg::{LoneLooseSegIndex, SeqLooseSegIndex},
    },
    graph::GetPetgraphIndex,
};

#[enum_dispatch]
pub trait GetPrevNextLoose {
    fn next_loose(&self, maybe_prev: Option<LooseIndex>) -> Option<LooseIndex>;

    fn prev_loose(&self, maybe_next: Option<LooseIndex>) -> Option<LooseIndex> {
        // `next_loose` and `prev_loose` do exactly the same thing
        // when `maybe_*` is `Some(_)`,
        // but otherwise, they start in opposite direction, here by going via:
        let maybe_prev = maybe_next.or_else(|| {
            let default_neighbor = self.next_loose(None);
            // * normally, one would retrieve the next element via `default_neighbor.next_loose(self)`
            // * `self.next_loose(default_neighbor)` on the other hand inverts the direction we're iterating towards.
            default_neighbor
        });
        self.next_loose(maybe_prev)
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

impl TryFrom<PrimitiveIndex> for LooseIndex {
    type Error = ();
    fn try_from(primitive: PrimitiveIndex) -> Result<LooseIndex, ()> {
        match primitive {
            PrimitiveIndex::LooseDot(dot) => Ok(dot.into()),
            PrimitiveIndex::LoneLooseSeg(seg) => Ok(seg.into()),
            PrimitiveIndex::SeqLooseSeg(seg) => Ok(seg.into()),
            PrimitiveIndex::LooseBend(bend) => Ok(bend.into()),
            _ => Err(()),
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

        if let Some(prev) = maybe_prev {
            if bend.petgraph_index() != prev.petgraph_index() {
                Some(bend.into())
            } else {
                self.seg().map(Into::into)
            }
        } else {
            Some(bend.into())
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

        if let Some(prev) = maybe_prev {
            if joints.0.petgraph_index() != prev.petgraph_index() {
                Some(joints.0.into())
            } else {
                Some(joints.1.into())
            }
        } else {
            Some(joints.0.into())
        }
    }
}
