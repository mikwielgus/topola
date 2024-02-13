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

    pub fn inner_bow_and_outer_bow(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let bend_primitive = self.layout.primitive(bend);
        let mut v = vec![];

        if let Some(inner) = bend_primitive.inner() {
            v.append(&mut self.bow(inner.into()));
        } else {
            let core = bend_primitive.core();
            v.push(core.into());
        }

        if let Some(outer) = bend_primitive.outer() {
            v.append(&mut self.bow(outer.into()));
        }

        v
    }

    pub fn inner_bow_and_outer_bows(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let bend_primitive = self.layout.primitive(bend);
        let mut v = vec![];

        if let Some(inner) = bend_primitive.inner() {
            v.append(&mut self.bow(inner.into()));
        } else {
            let core = bend_primitive.core();
            v.push(core.into());
        }

        let mut rail = bend;

        while let Some(outer) = self.layout.primitive(rail).outer() {
            v.append(&mut self.bow(outer.into()));
            rail = outer;
        }

        v
    }

    pub fn segbend_inner_and_outer_bibows(
        &self,
        from: DotIndex,
        around: WraparoundableIndex,
    ) -> Vec<GeometryIndex> {
        let mut v = match from {
            DotIndex::Fixed(..) => vec![],
            DotIndex::Loose(dot) => {
                self.inner_bow_and_outer_bow(self.layout.primitive(dot).bend().into())
            }
        };
        v.append(&mut self.this_and_wraparound_bow(around));
        v
    }

    pub fn this_and_wraparound_bow(&self, around: WraparoundableIndex) -> Vec<GeometryIndex> {
        let mut v = match around {
            WraparoundableIndex::FixedDot(..) => vec![around.into()],
            WraparoundableIndex::FixedBend(..) => vec![around.into()],
            WraparoundableIndex::LooseBend(bend) => self.bow(bend),
        };
        if let Some(wraparound) = self.layout.wraparoundable(around).wraparound() {
            v.append(&mut self.bow(wraparound));
        }
        v
    }

    pub fn bow(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
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

    pub fn outer_bows(&self, bend: LooseBendIndex) -> Vec<GeometryIndex> {
        let mut v = vec![];
        let mut rail = bend;

        while let Some(outer) = self.layout.primitive(rail).outer() {
            let primitive = self.layout.primitive(outer);

            v.push(outer.into());

            let ends = primitive.joints();
            v.push(ends.0.into());
            v.push(ends.1.into());

            v.push(self.layout.primitive(ends.0).seg().unwrap().into());
            v.push(self.layout.primitive(ends.1).seg().unwrap().into());

            rail = outer;
        }

        v
    }
}
