use crate::graph::{GenericIndex, GetPetgraphIndex};

use super::{
    band::{BandTerminatingSegIndex, BandUid},
    bend::LooseBendIndex,
    graph::PrimitiveIndex,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::{GetInnerOuter, GetJoints},
    rules::AccessRules,
    wraparoundable::{GetWraparound, WraparoundableIndex},
    Drawing,
};

#[derive(Debug)]
pub struct Collect<'a, CW: Copy, R: AccessRules> {
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW: Copy, R: AccessRules> Collect<'a, CW, R> {
    pub fn new(drawing: &'a Drawing<CW, R>) -> Self {
        Self { drawing }
    }

    pub fn loose_band_uid(&self, start_loose: LooseIndex) -> BandUid {
        BandUid::new(
            self.loose_band_first_seg(start_loose),
            self.loose_band_last_seg(start_loose),
        )
    }

    fn loose_band_first_seg(&self, start_loose: LooseIndex) -> BandTerminatingSegIndex {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return BandTerminatingSegIndex::Straight(seg);
        }

        let mut loose = start_loose;
        let mut prev = None;

        loop {
            if let Some(next_loose) = self.drawing.loose(loose).prev_loose(prev) {
                prev = Some(loose);
                loose = next_loose;
            } else {
                return BandTerminatingSegIndex::Bended(GenericIndex::new(loose.petgraph_index()));
            }
        }
    }

    fn loose_band_last_seg(&self, start_loose: LooseIndex) -> BandTerminatingSegIndex {
        if let LooseIndex::LoneSeg(seg) = start_loose {
            return BandTerminatingSegIndex::Straight(seg);
        }

        let mut loose = start_loose;
        let mut next = None;

        loop {
            if let Some(prev_loose) = self.drawing.loose(loose).next_loose(next) {
                next = Some(loose);
                loose = prev_loose;
            } else {
                return BandTerminatingSegIndex::Bended(GenericIndex::new(loose.petgraph_index()));
            }
        }
    }

    pub fn bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v: Vec<PrimitiveIndex> = vec![];
        v.push(bend.into());

        let joints = self.drawing.primitive(bend).joints();
        v.push(joints.0.into());
        v.push(joints.1.into());

        if let Some(seg0) = self.drawing.primitive(joints.0).seg() {
            v.push(seg0.into());
        }

        if let Some(seg1) = self.drawing.primitive(joints.1).seg() {
            v.push(seg1.into());
        }

        v
    }

    pub fn bend_outer_bows(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];
        let mut rail = bend;

        while let Some(outer) = self.drawing.primitive(rail).outer() {
            v.append(&mut self.bend_bow(outer.into()));
            rail = outer;
        }

        v
    }

    pub fn wraparounded_bows(&self, around: WraparoundableIndex) -> Vec<PrimitiveIndex> {
        let mut v = vec![];
        let mut rail = around.into();

        while let Some(outer) = self.drawing.wraparoundable(rail).wraparound() {
            let primitive = self.drawing.primitive(outer);

            v.push(outer.into());

            let joints = primitive.joints();
            v.push(joints.0.into());
            v.push(joints.1.into());

            v.push(self.drawing.primitive(joints.0).seg().unwrap().into());
            v.push(self.drawing.primitive(joints.1).seg().unwrap().into());

            rail = outer.into();
        }

        v
    }
}
