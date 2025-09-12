// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::{
    geometry::{shape::MeasureLength, GetLayer},
    graph::MakeRef,
};

use super::{
    graph::MakePrimitiveRef,
    loose::{GetPrevNextLoose, LooseIndex},
    primitive::MakePrimitiveShape,
    rules::AccessRules,
    seg::LooseSegIndex,
    Drawing,
};

pub type BandUid = planar_incr_embed::navmesh::OrderedPair<BandTermsegIndex>;
pub type BandTermsegIndex = LooseSegIndex;

impl<'a, CW: 'a, Cel: 'a, R: 'a> MakeRef<'a, Drawing<CW, Cel, R>> for BandTermsegIndex {
    type Output = BandRef<'a, CW, Cel, R>;
    fn ref_(&self, drawing: &'a Drawing<CW, Cel, R>) -> BandRef<'a, CW, Cel, R> {
        BandRef::new(*self, drawing)
    }
}

pub struct BandRef<'a, CW, Cel, R> {
    first_seg: BandTermsegIndex,
    drawing: &'a Drawing<CW, Cel, R>,
}

impl<'a, CW: 'a, Cel: 'a, R: 'a> BandRef<'a, CW, Cel, R> {
    pub fn new(
        first_seg: BandTermsegIndex,
        drawing: &'a Drawing<CW, Cel, R>,
    ) -> BandRef<'a, CW, Cel, R> {
        Self { first_seg, drawing }
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules> GetLayer for BandRef<'_, CW, Cel, R> {
    fn layer(&self) -> usize {
        self.first_seg.primitive_ref(self.drawing).layer()
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules> MeasureLength for BandRef<'_, CW, Cel, R> {
    fn length(&self) -> f64 {
        let mut maybe_loose: Option<LooseIndex> = Some(match self.first_seg {
            BandTermsegIndex::Lone(seg) => {
                return self.drawing.geometry().seg_shape(seg.into()).length();
            }
            BandTermsegIndex::Seq(first_loose_seg) => first_loose_seg.into(),
        });

        let mut prev = None;
        let mut length = 0.0;

        while let Some(loose) = maybe_loose {
            length += loose.primitive_ref(self.drawing).shape().length();

            let prev_prev = prev;
            prev = maybe_loose;
            maybe_loose = self.drawing.loose(loose).next_loose(prev_prev);
        }

        length
    }
}
