// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use petgraph::visit::Walker;

use super::{
    band::{BandTermsegIndex, BandUid},
    bend::LooseBendIndex,
    gear::WalkOutwards,
    graph::PrimitiveIndex,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::GetJoints,
    rules::AccessRules,
    Drawing,
};

#[derive(Clone, Debug, thiserror::Error)]
#[error("unable to resolve Loose to BandUid")]
pub struct BandUidError {
    pub maybe_end: Option<BandTermsegIndex>,
}

pub trait Collect {
    fn loose_band_uid(&self, start_loose: LooseIndex) -> Result<BandUid, BandUidError>;

    fn bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex>;

    fn bend_outward_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex>;
}

impl<CW: Clone, Cel: Copy, R: AccessRules> Collect for Drawing<CW, Cel, R> {
    fn loose_band_uid(&self, start_loose: LooseIndex) -> Result<BandUid, BandUidError> {
        match (
            self.loose_band_first_seg(start_loose),
            self.loose_band_last_seg(start_loose),
        ) {
            (Some(first), Some(last)) => Ok(BandUid::from((first, last))),
            (Some(x), None) | (None, Some(x)) => Err(BandUidError { maybe_end: Some(x) }),
            (None, None) => Err(BandUidError { maybe_end: None }),
        }
    }

    fn bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v: Vec<PrimitiveIndex> = vec![];
        v.push(bend.into());

        let joints = self.primitive(bend).joints();
        v.push(joints.0.into());
        v.push(joints.1.into());

        if let Some(seg0) = self.primitive(joints.0).seg() {
            v.push(seg0.into());
        }

        if let Some(seg1) = self.primitive(joints.1).seg() {
            v.push(seg1.into());
        }

        v
    }

    fn bend_outward_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];

        let mut outwards = self.primitive(bend).outwards();
        while let Some(next) = outwards.walk_next(self) {
            v.append(&mut self.bend_bow(next));
        }

        v
    }
}

trait CollectPrivate {
    fn loose_band_first_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex>;
    fn loose_band_last_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex>;
}

impl<CW: Clone, Cel: Copy, R: AccessRules> CollectPrivate for Drawing<CW, Cel, R> {
    fn loose_band_first_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex> {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return Some(BandTermsegIndex::Lone(seg));
        }

        let mut loose = start_loose;
        let mut prev = None;

        loop {
            if let Some(next_loose) = self.loose(loose).prev_loose(prev) {
                prev = Some(loose);
                loose = next_loose;
            } else {
                return loose.try_into().ok();
            }
        }
    }

    fn loose_band_last_seg(&self, start_loose: LooseIndex) -> Option<BandTermsegIndex> {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return Some(BandTermsegIndex::Lone(seg));
        }

        let mut loose = start_loose;
        let mut next = None;

        loop {
            if let Some(prev_loose) = self.loose(loose).next_loose(next) {
                next = Some(loose);
                loose = prev_loose;
            } else {
                return loose.try_into().ok();
            }
        }
    }
}
