use crate::wraparoundable::{GetWraparound, WraparoundableIndex};

use super::{
    bend::LooseBendIndex,
    dot::DotIndex,
    graph::GeometryIndex,
    primitive::{GetCore, GetInnerOuter, GetJoints},
    rules::RulesTrait,
    Layout,
};

#[derive(Debug)]
pub struct Collect<'a, R: RulesTrait> {
    layout: &'a Layout<R>,
}

impl<'a, R: RulesTrait> Collect<'a, R> {
    pub fn new(layout: &'a Layout<R>) -> Self {
        Self { layout }
    }

    pub fn bend_bow(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let mut v: Vec<GeometryIndex> = vec![];
        v.push(bend.into());

        let ends = self.layout.primitive(bend).joints();
        v.push(ends.0.into());
        v.push(ends.1.into());

        if let Some(seg0) = self.layout.primitive(ends.0).seg() {
            v.push(seg0.into());
        }

        if let Some(seg1) = self.layout.primitive(ends.1).seg() {
            v.push(seg1.into());
        }

        v
    }

    pub fn bend_outer_bows(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let mut v = vec![];
        let mut rail = bend;

        while let Some(outer) = self.layout.primitive(rail).outer() {
            v.append(&mut self.bend_bow(outer.into()));
            rail = outer;
        }

        v
    }

    pub fn wraparounded_bows(&self, around: WraparoundableIndex) -> Vec<GeometryIndex> {
        let mut v = vec![];
        let mut rail = around.into();

        while let Some(outer) = self.layout.wraparoundable(rail).wraparound() {
            let primitive = self.layout.primitive(outer);

            v.push(outer.into());

            let ends = primitive.joints();
            v.push(ends.0.into());
            v.push(ends.1.into());

            v.push(self.layout.primitive(ends.0).seg().unwrap().into());
            v.push(self.layout.primitive(ends.1).seg().unwrap().into());

            rail = outer.into();
        }

        v
    }
}
