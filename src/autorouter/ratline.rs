// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use geo::{line_intersection::line_intersection, Distance, Euclidean, Line, LineIntersection};
use petgraph::graph::{EdgeIndex, NodeIndex};
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

    pub fn closure_obstacle_ratlines(&self) -> impl Iterator<Item = RatlineIndex> + '_ {
        self.intersecting_ratlines()
    }

    pub fn interior_obstacle_ratlines(&self) -> impl Iterator<Item = RatlineIndex> + '_ {
        self.intersecting_ratlines()
            .filter(|index| !self.is_pin_cutter(*index))
    }

    fn intersecting_ratlines(&self) -> impl Iterator<Item = RatlineIndex> + '_ {
        let self_line_segment = self.line_segment();

        self.autorouter
            .ratsnest()
            .graph()
            .edge_indices()
            .filter(move |other| {
                let (self_source, self_target) = self.endpoint_indices();
                let (other_source, other_target) = other.ref_(self.autorouter).endpoint_indices();

                self_source != other_source
                    && self_source != other_target
                    && self_target != other_source
                    && self_target != other_target
            })
            .filter(move |other| {
                let other_line_segment = other.ref_(self.autorouter).line_segment();

                line_intersection(self_line_segment, other_line_segment).is_some()
            })
    }

    fn is_pin_cutter(&self, other: RatlineIndex) -> bool {
        // TODO: For now, instead of detecting whether endpoint ratvertex pins
        // are cut, we only check if the intersection between self and the
        // supposed cutter is not internal.

        let self_line_segment = self.line_segment();
        let other_line_segment = other.ref_(self.autorouter).line_segment();

        if let Some(LineIntersection::SinglePoint { is_proper, .. }) =
            line_intersection(self_line_segment, other_line_segment)
        {
            // It would make more sense to check for non-internality only in
            // self, but this gives me the result I want too for now.
            !is_proper
        } else {
            false
        }
    }

    pub fn line_segment(&self) -> Line {
        let (source, target) = self.endpoint_indices();
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

    fn endpoint_indices(&self) -> (NodeIndex<usize>, NodeIndex<usize>) {
        self.autorouter
            .ratsnest
            .graph()
            .edge_endpoints(self.index)
            .unwrap()
    }
}

impl<'a, M: AccessMesadata> MeasureLength for RatlineRef<'a, M> {
    fn length(&self) -> f64 {
        let line = self.line_segment();

        Euclidean::distance(&line.start_point(), &line.end_point())
    }
}
