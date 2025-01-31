// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::graph::{GenericIndex, GetPetgraphIndex, MakeRef};

use super::{
    band::{BandTermsegIndex, BandUid},
    bend::LooseBendIndex,
    gear::{GearIndex, GetNextGear},
    graph::PrimitiveIndex,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::{GetInnerOuter, GetJoints},
    rules::AccessRules,
    Drawing,
};

pub trait Collect {
    fn loose_band_uid(&self, start_loose: LooseIndex) -> BandUid;

    fn bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex>;

    fn bend_outer_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex>;

    fn wraparounded_bows(&self, around: GearIndex) -> Vec<PrimitiveIndex>;
}

impl<CW: Copy, R: AccessRules> Collect for Drawing<CW, R> {
    fn loose_band_uid(&self, start_loose: LooseIndex) -> BandUid {
        BandUid::from((
            self.loose_band_first_seg(start_loose),
            self.loose_band_last_seg(start_loose),
        ))
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

    fn bend_outer_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];
        let mut gear = bend;

        while let Some(outer) = self.primitive(gear).outer() {
            v.append(&mut self.bend_bow(outer));
            gear = outer;
        }

        v
    }

    fn wraparounded_bows(&self, around: GearIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];
        let mut gear = around;

        while let Some(bend) = gear.ref_(self).next_gear() {
            let primitive = self.primitive(bend);

            v.push(bend.into());

            let joints = primitive.joints();
            v.push(joints.0.into());
            v.push(joints.1.into());

            v.push(self.primitive(joints.0).seg().unwrap().into());
            v.push(self.primitive(joints.1).seg().unwrap().into());

            gear = bend.into();
        }

        v
    }
}

trait CollectPrivate {
    fn loose_band_first_seg(&self, start_loose: LooseIndex) -> BandTermsegIndex;
    fn loose_band_last_seg(&self, start_loose: LooseIndex) -> BandTermsegIndex;
}

impl<CW: Copy, R: AccessRules> CollectPrivate for Drawing<CW, R> {
    fn loose_band_first_seg(&self, start_loose: LooseIndex) -> BandTermsegIndex {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return BandTermsegIndex::Straight(seg);
        }

        let mut loose = start_loose;
        let mut prev = None;

        loop {
            if let Some(next_loose) = self.loose(loose).prev_loose(prev) {
                prev = Some(loose);
                loose = next_loose;
            } else {
                return BandTermsegIndex::Bended(GenericIndex::new(loose.petgraph_index()));
            }
        }
    }

    fn loose_band_last_seg(&self, start_loose: LooseIndex) -> BandTermsegIndex {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return BandTermsegIndex::Straight(seg);
        }

        let mut loose = start_loose;
        let mut next = None;

        loop {
            if let Some(prev_loose) = self.loose(loose).next_loose(next) {
                next = Some(loose);
                loose = prev_loose;
            } else {
                return BandTermsegIndex::Bended(GenericIndex::new(loose.petgraph_index()));
            }
        }
    }
}
