use crate::{
    connectivity::{BandIndex, BandWeight, ConnectivityWeight, GetNet},
    geometry::{DotIndex, FixedDotIndex},
    graph::GetNodeIndex,
    layout::Layout,
    loose::{GetNextLoose, LooseIndex},
    primitive::GetEnds,
};

pub struct Band<'a> {
    pub index: BandIndex,
    layout: &'a Layout,
}

impl<'a> Band<'a> {
    pub fn new(index: BandIndex, layout: &'a Layout) -> Self {
        Self { index, layout }
    }

    fn weight(&self) -> BandWeight {
        if let Some(ConnectivityWeight::Band(weight)) = self
            .layout
            .connectivity()
            .node_weight(self.index.node_index())
        {
            *weight
        } else {
            unreachable!()
        }
    }

    pub fn from(&self) -> FixedDotIndex {
        self.weight().from
    }

    pub fn to(&self) -> Option<FixedDotIndex> {
        // For now, we do full traversal. Later on, we may want to store the target fixed dot
        // somewhere.

        let mut maybe_loose = self.layout.primitive(self.from()).first_loose(self.index);
        let mut prev = None;

        while let Some(loose) = maybe_loose {
            let prev_prev = prev;
            prev = maybe_loose;
            maybe_loose = self.layout.loose(loose).next_loose(prev_prev);
        }

        if let Some(LooseIndex::SeqSeg(seg)) = maybe_loose {
            if let DotIndex::Fixed(dot) = self.layout.primitive(seg).ends().0 {
                Some(dot)
            } else {
                unreachable!()
            }
        } else {
            None
        }
    }
}

impl<'a> GetNet for Band<'a> {
    fn net(&self) -> i64 {
        self.weight().net
    }
}
