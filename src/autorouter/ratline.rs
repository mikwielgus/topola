// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use geo::{line_intersection::line_intersection, Distance, Euclidean, Line};
use petgraph::graph::EdgeIndex;
use specctra_core::mesadata::AccessMesadata;

use crate::{
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex},
    geometry::shape::MeasureLength,
    graph::MakeRef,
    triangulation::GetTrianvertexNodeIndex,
};

use super::{ratsnest::RatvertexIndex, Autorouter};

pub type RatlineIndex = EdgeIndex<usize>;

#[derive(Debug, Default, Clone, Copy)]
pub struct RatlineWeight {
    pub band_termseg: Option<BandTermsegIndex>,
}

impl<'a, M: AccessMesadata + 'a> MakeRef<'a, Autorouter<M>> for RatlineIndex {
    type Output = RatlineRef<'a, M>;
    fn ref_(&self, autorouter: &'a Autorouter<M>) -> RatlineRef<'a, M> {
        RatlineRef::new(*self, autorouter)
    }
}

pub struct RatlineRef<'a, M: AccessMesadata> {
    index: RatlineIndex,
    autorouter: &'a Autorouter<M>,
}

impl<'a, M: AccessMesadata> RatlineRef<'a, M> {
    pub fn new(index: RatlineIndex, autorouter: &'a Autorouter<M>) -> Self {
        Self { index, autorouter }
    }

    pub fn endpoint_dots(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (source, target) = self
            .autorouter
            .ratsnest
            .graph()
            .edge_endpoints(self.index)
            .unwrap();

        let source_dot = match self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(source)
            .unwrap()
            .node_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Poly(poly) => poly.ref_(self.autorouter.board.layout()).apex(),
        };

        let target_dot = match self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .node_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Poly(poly) => poly.ref_(self.autorouter.board.layout()).apex(),
        };

        (source_dot, target_dot)
    }

    pub fn find_intersecting_ratlines(&self) -> impl Iterator<Item = RatlineIndex> + '_ {
        let self_line = self.line();

        self.autorouter
            .ratsnest()
            .graph()
            .edge_indices()
            .filter(move |other| {
                let (self_source, self_target) = self
                    .autorouter
                    .ratsnest
                    .graph()
                    .edge_endpoints(self.index)
                    .unwrap();
                let (other_source, other_target) = self
                    .autorouter
                    .ratsnest
                    .graph()
                    .edge_endpoints(*other)
                    .unwrap();

                self_source != other_source
                    && self_source != other_target
                    && self_target != other_source
                    && self_target != other_target
            })
            .filter(move |other| {
                let other_line = other.ref_(self.autorouter).line();

                line_intersection(self_line, other_line).is_some()
            })
    }

    pub fn line(&self) -> Line {
        let (source, target) = self
            .autorouter
            .ratsnest
            .graph()
            .edge_endpoints(self.index)
            .unwrap();
        let source_pos = self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(source)
            .unwrap()
            .pos;
        let target_pos = self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .pos;

        Line::new(source_pos, target_pos)
    }
}

impl<'a, M: AccessMesadata> MeasureLength for RatlineRef<'a, M> {
    fn length(&self) -> f64 {
        let line = self.line();

        Euclidean::distance(&line.start_point(), &line.end_point())
    }
}
