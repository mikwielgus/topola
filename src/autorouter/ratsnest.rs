// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{data::Element, graph::EdgeIndex, prelude::StableUnGraph};
use spade::{handles::FixedVertexHandle, HasPosition, InsertionError, Point2};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::connected_components::ConnectedComponents,
    board::Board,
    drawing::{
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitiveRef, PrimitiveIndex},
        primitive::MakePrimitiveShape,
    },
    geometry::shape::AccessShape,
    graph::{GenericIndex, GetIndex, MakeRef},
    layout::poly::{MakePolygon, PolyWeight},
    triangulation::{GetTrianvertexNodeIndex, Triangulation},
};

use super::ratline::RatlineWeight;

#[enum_dispatch(GetIndex)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatvertexNodeIndex {
    FixedDot(FixedDotIndex),
    Poly(GenericIndex<PolyWeight>),
}

impl From<RatvertexNodeIndex> for crate::layout::NodeIndex {
    fn from(vertex: RatvertexNodeIndex) -> crate::layout::NodeIndex {
        match vertex {
            RatvertexNodeIndex::FixedDot(dot) => crate::layout::NodeIndex::Primitive(dot.into()),
            RatvertexNodeIndex::Poly(poly) => crate::layout::NodeIndex::Compound(poly.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RatvertexWeight {
    vertex: RatvertexNodeIndex,
    pub pos: Point,
}

impl GetTrianvertexNodeIndex<RatvertexNodeIndex> for RatvertexWeight {
    fn node_index(&self) -> RatvertexNodeIndex {
        self.vertex
    }
}

impl HasPosition for RatvertexWeight {
    type Scalar = f64;
    fn position(&self) -> Point2<Self::Scalar> {
        Point2::new(self.pos.x(), self.pos.y())
    }
}

#[derive(Clone)]
struct RatvertexToHandleMap {
    fixed_dot_to_handle: Box<[Option<FixedVertexHandle>]>,
    poly_to_handle: Box<[Option<FixedVertexHandle>]>,
}

impl RatvertexToHandleMap {
    pub fn new(fixed_dot_bound: usize, poly_bound: usize) -> Self {
        Self {
            fixed_dot_to_handle: vec![None; fixed_dot_bound].into_boxed_slice(),
            poly_to_handle: vec![None; poly_bound].into_boxed_slice(),
        }
    }
}

impl Index<RatvertexNodeIndex> for RatvertexToHandleMap {
    type Output = Option<FixedVertexHandle>;

    fn index(&self, ratvertex: RatvertexNodeIndex) -> &Self::Output {
        match ratvertex {
            RatvertexNodeIndex::FixedDot(dot) => &self.fixed_dot_to_handle[dot.index()],
            RatvertexNodeIndex::Poly(bend) => &self.poly_to_handle[bend.index()],
        }
    }
}

impl IndexMut<RatvertexNodeIndex> for RatvertexToHandleMap {
    fn index_mut(&mut self, ratvertex: RatvertexNodeIndex) -> &mut Self::Output {
        match ratvertex {
            RatvertexNodeIndex::FixedDot(dot) => &mut self.fixed_dot_to_handle[dot.index()],
            RatvertexNodeIndex::Poly(bend) => &mut self.poly_to_handle[bend.index()],
        }
    }
}

pub struct Ratsnest {
    graph: StableUnGraph<RatvertexWeight, RatlineWeight, usize>,
}

impl Ratsnest {
    pub fn new(
        board: &Board<impl AccessMesadata>,
        principal_layer: usize,
    ) -> Result<Self, InsertionError> {
        let conncomps = ConnectedComponents::new_with_principal_layer(board, principal_layer);

        let mut this = Self {
            graph: StableUnGraph::default(),
        };

        let mut triangulations = BTreeMap::new();

        this.add_layer_to_ratsnest_triangulations(board, &mut triangulations, principal_layer);

        for layer in 0..board.layout().drawing().layer_count() {
            if layer != principal_layer {
                this.add_layer_to_ratsnest_triangulations(board, &mut triangulations, layer);
            }
        }

        for (_net, triangulation) in triangulations {
            let mut map = Vec::new();

            for element in petgraph::algo::min_spanning_tree(&triangulation) {
                match element {
                    Element::Node { weight } => {
                        map.push(this.graph.add_node(weight));
                    }
                    Element::Edge {
                        source,
                        target,
                        weight,
                    } => {
                        this.graph.add_edge(map[source], map[target], weight.weight);
                    }
                }
            }
        }

        this.graph.retain_edges(|g, i| {
            if let Some((source, target)) = g.edge_endpoints(i) {
                let source_index = g.node_weight(source).unwrap().node_index().index();
                let target_index = g.node_weight(target).unwrap().node_index().index();
                !conncomps.unionfind().equiv(source_index, target_index)
            } else {
                true
            }
        });

        Ok(this)
    }

    fn add_layer_to_ratsnest_triangulations(
        &mut self,
        board: &Board<impl AccessMesadata>,
        triangulations: &mut BTreeMap<
            usize,
            Triangulation<RatvertexNodeIndex, RatvertexToHandleMap, RatvertexWeight, RatlineWeight>,
        >,
        layer: usize,
    ) -> Result<(), InsertionError> {
        let mut handle_ratvertex_weight =
            |maybe_net: Option<usize>, vertex: RatvertexNodeIndex, pos: Point| {
                let Some(net) = maybe_net else {
                    return Ok(());
                };

                let triangulation = triangulations.entry(net).or_insert_with(|| {
                    Triangulation::new(RatvertexToHandleMap::new(
                        board.layout().drawing().geometry().dot_index_bound(),
                        board.layout().drawing().geometry().compound_index_bound(),
                    ))
                });

                // If a vertex already exists at `pos` for this net, skip.
                // This should prevent overwriting principal layer vertices
                // with ones from non-principal layers.
                if triangulation.find_vertex_at_position(pos).is_some() {
                    return Ok(());
                }

                triangulation.add_vertex(RatvertexWeight { vertex, pos })?;
                Ok(())
            };

        for node in board.layout().drawing().layer_primitive_nodes(layer) {
            if let PrimitiveIndex::FixedDot(dot) = node {
                // Dots that are parts of polys are ignored because ratlines
                // should only go to their centerpoints.
                if board.layout().drawing().compounds(dot).next().is_none() {
                    handle_ratvertex_weight(
                        board.layout().drawing().primitive(dot).maybe_net(),
                        RatvertexNodeIndex::FixedDot(dot),
                        node.primitive_ref(board.layout().drawing())
                            .shape()
                            .center(),
                    )?;
                }
            }
        }

        for poly in board.layout().layer_poly_nodes(layer) {
            handle_ratvertex_weight(
                board
                    .layout()
                    .drawing()
                    .compound_weight(poly.into())
                    .maybe_net(),
                RatvertexNodeIndex::Poly(poly),
                poly.ref_(board.layout()).shape().center(),
            )?;
        }

        Ok(())
    }

    pub fn assign_band_termseg_to_ratline(
        &mut self,
        ratline_index: EdgeIndex<usize>,
        termseg: BandTermsegIndex,
    ) {
        self.graph
            .edge_weight_mut(ratline_index)
            .unwrap()
            .band_termseg = Some(termseg);
    }

    pub fn graph(&self) -> &StableUnGraph<RatvertexWeight, RatlineWeight, usize> {
        &self.graph
    }
}
