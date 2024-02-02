use crate::{
    graph::GetNodeIndex,
    layout::{
        connectivity::{BandIndex, BandWeight, ConnectivityWeight, GetNet},
        dot::{DotIndex, FixedDotIndex},
        geometry::shape::ShapeTrait,
        graph::{GeometryIndex, MakePrimitive},
        loose::{GetNextLoose, LooseIndex},
        primitive::{GetJoints, GetOtherJoint, MakeShape},
        Layout,
    },
};

use super::rules::RulesTrait;

pub struct Band<'a, R: RulesTrait> {
    pub index: BandIndex,
    layout: &'a Layout<R>,
}

impl<'a, R: RulesTrait> Band<'a, R> {
    pub fn new(index: BandIndex, layout: &'a Layout<R>) -> Self {
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
                Some(self.layout.primitive(seg).other_joint(self.from()))
            }
            Some(LooseIndex::SeqSeg(seg)) => {
                if let DotIndex::Fixed(dot) = self.layout.primitive(seg).joints().0 {
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

impl<'a, R: RulesTrait> GetNet for Band<'a, R> {
    fn net(&self) -> i64 {
        self.weight().net
    }
}
