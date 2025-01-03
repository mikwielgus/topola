// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::{cmp, hash};
use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::NodeIndex;

use crate::{
    geometry::shape::MeasureLength,
    graph::{GetPetgraphIndex, MakeRef},
};

use super::{
    graph::MakePrimitive,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::MakePrimitiveShape,
    rules::AccessRules,
    seg::{LoneLooseSegIndex, SeqLooseSegIndex},
    Drawing,
};

#[derive(Clone, Copy, Debug)]
pub struct BandUid(pub BandTermsegIndex, pub BandTermsegIndex);

impl BandUid {
    pub fn new(first_seg1: BandTermsegIndex, first_seg2: BandTermsegIndex) -> Self {
        if first_seg1.petgraph_index() <= first_seg2.petgraph_index() {
            BandUid(first_seg1, first_seg2)
        } else {
            BandUid(first_seg2, first_seg1)
        }
    }
}

impl PartialEq for BandUid {
    fn eq(&self, other: &Self) -> bool {
        self.0.petgraph_index() == other.0.petgraph_index()
            && self.1.petgraph_index() == other.1.petgraph_index()
    }
}

impl Eq for BandUid {}

impl hash::Hash for BandUid {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.petgraph_index().hash(state);
        self.1.petgraph_index().hash(state);
    }
}

impl cmp::PartialOrd for BandUid {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for BandUid {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        use cmp::Ordering as O;
        match self.0.petgraph_index().cmp(&other.0.petgraph_index()) {
            O::Less => O::Less,
            O::Greater => O::Greater,
            O::Equal => self.1.petgraph_index().cmp(&other.1.petgraph_index()),
        }
    }
}

#[enum_dispatch(GetPetgraphIndex)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BandTermsegIndex {
    Straight(LoneLooseSegIndex),
    Bended(SeqLooseSegIndex),
}

impl From<BandTermsegIndex> for LooseIndex {
    fn from(terminating_seg: BandTermsegIndex) -> Self {
        match terminating_seg {
            BandTermsegIndex::Straight(seg) => LooseIndex::LoneSeg(seg),
            BandTermsegIndex::Bended(seg) => LooseIndex::SeqSeg(seg),
        }
    }
}

impl<'a, CW, R> MakeRef<'a, BandRef<'a, CW, R>, Drawing<CW, R>> for BandTermsegIndex {
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> BandRef<'a, CW, R> {
        BandRef::new(*self, drawing)
    }
}

pub struct BandRef<'a, CW, R> {
    first_seg: BandTermsegIndex,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW, R> BandRef<'a, CW, R> {
    pub fn new(first_seg: BandTermsegIndex, drawing: &'a Drawing<CW, R>) -> BandRef<'a, CW, R> {
        Self { first_seg, drawing }
    }
}

impl<'a, CW: Copy, R: AccessRules> MeasureLength for BandRef<'a, CW, R> {
    fn length(&self) -> f64 {
        match self.first_seg {
            BandTermsegIndex::Straight(seg) => {
                self.drawing.geometry().seg_shape(seg.into()).length()
            }
            BandTermsegIndex::Bended(first_loose_seg) => {
                let mut maybe_loose: Option<LooseIndex> = Some(first_loose_seg.into());
                let mut prev = None;
                let mut length = 0.0;

                while let Some(loose) = maybe_loose {
                    length += loose.primitive(self.drawing).shape().length();

                    let prev_prev = prev;
                    prev = maybe_loose;
                    maybe_loose = self.drawing.loose(loose).next_loose(prev_prev);
                }

                length
            }
        }
    }
}
