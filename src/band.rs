use crate::{
    connectivity::{BandIndex, BandWeight, ConnectivityWeight, GetNet},
    geometry::{
        dot::{DotIndex, FixedDotIndex},
        geometry::{GeometryIndex, MakePrimitive},
    },
    graph::GetNodeIndex,
    layout::Layout,
    loose::{GetNextLoose, LooseIndex},
    primitive::{GetEnds, GetOtherEnd, MakeShape},
    shape::ShapeTrait,
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

        match prev {
            Some(LooseIndex::LoneSeg(seg)) => {
                Some(self.layout.primitive(seg).other_end(self.from()))
            }
            Some(LooseIndex::SeqSeg(seg)) => {
                if let DotIndex::Fixed(dot) = self.layout.primitive(seg).ends().0 {
                    Some(dot)
                } else {
                    None
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn length(&self) -> f64 {
        let mut maybe_loose = self.layout.primitive(self.from()).first_loose(self.index);
        let mut prev = None;
        let mut length = 0.0;

        while let Some(loose) = maybe_loose {
            length += GeometryIndex::from(loose)
                .primitive(self.layout)
                .shape()
                .length();

            let prev_prev = prev;
            prev = maybe_loose;
            maybe_loose = self.layout.loose(loose).next_loose(prev_prev);
        }

        length
    }
}

impl<'a> GetNet for Band<'a> {
    fn net(&self) -> i64 {
        self.weight().net
    }
}
