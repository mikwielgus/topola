// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::{line_intersection::line_intersection, Distance, Euclidean, Line, LineIntersection};
use petgraph::graph::{EdgeIndex, NodeIndex};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{ratsnest::RatvertexNodeIndex, Autorouter},
    drawing::{
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitiveRef, PrimitiveIndex},
    },
    geometry::shape::MeasureLength,
    graph::MakeRef,
    triangulation::GetTrianvertexNodeIndex,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RatlineUid {
    pub principal_layer: usize,
    pub index: EdgeIndex<usize>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RatlineWeight {
    pub layer: usize,
    pub band_termseg: Option<BandTermsegIndex>,
}

impl<'a, M: AccessMesadata + 'a> MakeRef<'a, Autorouter<M>> for RatlineUid {
    type Output = RatlineRef<'a, M>;
    fn ref_(&self, autorouter: &'a Autorouter<M>) -> RatlineRef<'a, M> {
        RatlineRef::new(*self, autorouter)
    }
}

pub struct RatlineRef<'a, M: AccessMesadata> {
    uid: RatlineUid,
    autorouter: &'a Autorouter<M>,
}

impl<'a, M: AccessMesadata> RatlineRef<'a, M> {
    pub fn new(index: RatlineUid, autorouter: &'a Autorouter<M>) -> Self {
        Self {
            uid: index,
            autorouter,
        }
    }

    pub fn band_termseg(&self) -> BandTermsegIndex {
        self.autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .edge_weight(self.uid.index)
            .unwrap()
            .band_termseg
            .unwrap()
    }

    pub fn endpoint_dots(&self) -> (FixedDotIndex, FixedDotIndex) {
        let (source, target) = self
            .autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .edge_endpoints(self.uid.index)
            .unwrap();

        let source_dot = match self
            .autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .node_weight(source)
            .unwrap()
            .node_index()
        {
            RatvertexNodeIndex::FixedDot(dot) => dot,
            RatvertexNodeIndex::Poly(poly) => poly.ref_(self.autorouter.board.layout()).apex(),
        };

        let target_dot = match self
            .autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .node_weight(target)
            .unwrap()
            .node_index()
        {
            RatvertexNodeIndex::FixedDot(dot) => dot,
            RatvertexNodeIndex::Poly(poly) => poly.ref_(self.autorouter.board.layout()).apex(),
        };

        (source_dot, target_dot)
    }

    pub fn layer(&self) -> usize {
        self.autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .edge_weight(self.uid.index)
            .unwrap()
            .layer
    }

    pub fn net(&self) -> usize {
        self.endpoint_dots()
            .0
            .primitive_ref(self.autorouter.board().layout().drawing())
            .maybe_net()
            .unwrap()
    }

    fn cut_primitives(&self) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.autorouter
            .board()
            .layout()
            .drawing()
            .cut(self.line_segment(), 0.0, self.layer())
    }

    pub fn cut_other_net_primitives(&self) -> impl Iterator<Item = PrimitiveIndex> + '_ {
        self.cut_primitives().filter(|primitive_node| {
            primitive_node
                .primitive_ref(self.autorouter.board().layout().drawing())
                .maybe_net()
                .map(|net| net != self.net())
                .unwrap_or(true)
        })
    }

    pub fn interiorly_cut_ratlines(&self) -> impl Iterator<Item = RatlineUid> + '_ {
        let self_line_segment = self.line_segment();

        self.autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .edge_indices()
            .map(|index| RatlineUid {
                principal_layer: self.uid.principal_layer,
                index,
            })
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

                if let Some(LineIntersection::SinglePoint { is_proper, .. }) =
                    line_intersection(self_line_segment, other_line_segment)
                {
                    // It would make more sense to check for non-internality only in
                    // self, but this gives me the result I want too for now.
                    !is_proper
                } else {
                    false
                }
            })
    }

    pub fn line_segment(&self) -> Line {
        let (source, target) = self.endpoint_indices();
        let source_pos = self
            .autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .node_weight(source)
            .unwrap()
            .pos;
        let target_pos = self
            .autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .node_weight(target)
            .unwrap()
            .pos;

        Line::new(source_pos, target_pos)
    }

    pub fn endpoint_indices(&self) -> (NodeIndex<usize>, NodeIndex<usize>) {
        self.autorouter
            .ratsnests()
            .on_principal_layer(self.uid.principal_layer)
            .graph()
            .edge_endpoints(self.uid.index)
            .unwrap()
    }
}

impl<'a, M: AccessMesadata> MeasureLength for RatlineRef<'a, M> {
    fn length(&self) -> f64 {
        let line = self.line_segment();

        Euclidean::distance(&line.start_point(), &line.end_point())
    }
}
