use crate::wraparoundable::{GetWraparound, WraparoundableIndex};

use super::{
    bend::LooseBendIndex,
    graph::PrimitiveIndex,
    primitive::{GetInnerOuter, GetJoints},
    rules::RulesTrait,
    Drawing,
};

#[derive(Debug)]
pub struct Collect<'a, GW: Copy, R: RulesTrait> {
    drawing: &'a Drawing<GW, R>,
}

impl<'a, GW: Copy, R: RulesTrait> Collect<'a, GW, R> {
    pub fn new(drawing: &'a Drawing<GW, R>) -> Self {
        Self { drawing }
    }

    pub fn bend_bow(&self, bend: LooseBendIndex) -> Vec<PrimitiveIndex> {
        let mut v: Vec<PrimitiveIndex> = vec![];
        v.push(bend.into());

        let ends = self.drawing.primitive(bend).joints();
        v.push(ends.0.into());
        v.push(ends.1.into());

        if let Some(seg0) = self.drawing.primitive(ends.0).seg() {
            v.push(seg0.into());
        }

        if let Some(seg1) = self.drawing.primitive(ends.1).seg() {
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

            let ends = primitive.joints();
            v.push(ends.0.into());
            v.push(ends.1.into());

            v.push(self.drawing.primitive(ends.0).seg().unwrap().into());
            v.push(self.drawing.primitive(ends.1).seg().unwrap().into());

            rail = outer.into();
        }

        v
    }
}
