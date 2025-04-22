// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

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

pub type BandUid = planar_incr_embed::navmesh::OrderedPair<BandTermsegIndex>;

#[enum_dispatch(GetPetgraphIndex)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
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

impl<'a, CW: 'a, Cek: 'a, R: 'a> MakeRef<'a, Drawing<CW, Cek, R>> for BandTermsegIndex {
    type Output = BandRef<'a, CW, Cek, R>;
    fn ref_(&self, drawing: &'a Drawing<CW, Cek, R>) -> BandRef<'a, CW, Cek, R> {
        BandRef::new(*self, drawing)
    }
}

pub struct BandRef<'a, CW, Cek, R> {
    first_seg: BandTermsegIndex,
    drawing: &'a Drawing<CW, Cek, R>,
}

impl<'a, CW: 'a, Cek: 'a, R: 'a> BandRef<'a, CW, Cek, R> {
    pub fn new(
        first_seg: BandTermsegIndex,
        drawing: &'a Drawing<CW, Cek, R>,
    ) -> BandRef<'a, CW, Cek, R> {
        Self { first_seg, drawing }
    }
}

impl<CW: Clone, Cek: Copy, R: AccessRules> MeasureLength for BandRef<'_, CW, Cek, R> {
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
