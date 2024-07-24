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

#[derive(Debug, Hash, Clone, Copy)]
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

#[enum_dispatch(GetPetgraphIndex)]
#[derive(Debug, Hash, Clone, Copy)]
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

impl<'a, CW: Copy, R: AccessRules> MakeRef<'a, BandRef<'a, CW, R>, Drawing<CW, R>>
    for BandTermsegIndex
{
    fn ref_(&self, drawing: &'a Drawing<CW, R>) -> BandRef<'a, CW, R> {
        BandRef::new(*self, drawing)
    }
}

pub struct BandRef<'a, CW: Copy, R: AccessRules> {
    first_seg: BandTermsegIndex,
    drawing: &'a Drawing<CW, R>,
}

impl<'a, CW: Copy, R: AccessRules> BandRef<'a, CW, R> {
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
