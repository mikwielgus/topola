// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use geo::{Distance, Euclidean};
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
}

impl<'a, M: AccessMesadata> MeasureLength for RatlineRef<'a, M> {
    fn length(&self) -> f64 {
        let (ratvertex0, ratvertex1) = self
            .autorouter
            .ratsnest
            .graph()
            .edge_endpoints(self.index)
            .unwrap();
        let ratvertex0_pos = self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(ratvertex0)
            .unwrap()
            .pos;
        let ratvertex1_pos = self
            .autorouter
            .ratsnest
            .graph()
            .node_weight(ratvertex1)
            .unwrap()
            .pos;

        Euclidean::distance(&ratvertex0_pos, &ratvertex1_pos)
    }
}
