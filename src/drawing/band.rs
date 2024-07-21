use crate::{
    drawing::seg::{LoneLooseSegIndex, SeqLooseSegIndex},
    geometry::shape::MeasureLength,
    graph::MakeRef,
};

use super::{
    dot::DotIndex,
    primitive::{GetJoints, GetOtherJoint},
    rules::AccessRules,
    Drawing,
};

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum BandFirstSegIndex {
    Straight(LoneLooseSegIndex),
    Bended(SeqLooseSegIndex),
}

impl<'a, CW: Copy, R: AccessRules> MakeRef<'a, BandRef<'a, CW, R>, Drawing<CW, R>>
    for BandFirstSegIndex
{
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> BandRef<'a, CW, R> {
        BandRef::new(*self, drawing)
    }
}

pub struct BandRef<'a, CW: Copy, R: AccessRules> {
    first_seg: BandFirstSegIndex,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW: Copy, R: AccessRules> BandRef<'a, CW, R> {
    pub fn new(first_seg: BandFirstSegIndex, drawing: &'a Drawing<CW, R>) -> BandRef<'a, CW, R> {
        Self { first_seg, drawing }
    }
}

impl<'a, CW: Copy, R: AccessRules> MeasureLength for BandRef<'a, CW, R> {
    fn length(&self) -> f64 {
        match self.first_seg {
            BandFirstSegIndex::Straight(seg) => {
                self.drawing.geometry().seg_shape(seg.into()).length()
            }
            BandFirstSegIndex::Bended(start_seg) => {
                let mut length = self.drawing.geometry().seg_shape(start_seg.into()).length();
                let start_dot = self.drawing.primitive(start_seg).joints().1;

                let bend = self.drawing.primitive(start_dot).bend();
                length += self.drawing.geometry().bend_shape(bend.into()).length();

                let mut prev_dot = self.drawing.primitive(bend).other_joint(start_dot.into());
                let mut seg = self.drawing.primitive(prev_dot).seg().unwrap();
                length += self.drawing.geometry().seg_shape(seg.into()).length();

                while let DotIndex::Loose(dot) =
                    self.drawing.primitive(seg).other_joint(prev_dot.into())
                {
                    let bend = self.drawing.primitive(dot).bend();
                    length += self.drawing.geometry().bend_shape(bend.into()).length();

                    prev_dot = self.drawing.primitive(bend).other_joint(dot);
                    seg = self.drawing.primitive(prev_dot).seg().unwrap();
                    length += self.drawing.geometry().seg_shape(seg.into()).length();
                }

                length
            }
        }
    }
}
