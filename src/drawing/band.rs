use crate::{geometry::shape::MeasureLength, graph::MakeRef};

use super::{
    graph::MakePrimitive,
    loose::{GetNextLoose, LooseIndex},
    primitive::{GetJoints, GetOtherJoint, MakePrimitiveShape},
    rules::AccessRules,
    seg::{LoneLooseSegIndex, SeqLooseSegIndex},
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
            BandFirstSegIndex::Bended(first_loose_seg) => {
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
