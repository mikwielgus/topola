// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use geo::point;
use petgraph::graph::NodeIndex;
use rstar::{Envelope, RTreeObject, AABB};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{ratline::RatlineIndex, Autorouter},
    board::edit::BoardEdit,
    drawing::{
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitiveRef},
        primitive::MakePrimitiveShape,
    },
    geometry::{primitive::PrimitiveShape, shape::AccessShape, GenericNode, GetLayer},
    graph::{GenericIndex, GetIndex, MakeRef},
    layout::{
        poly::{MakePolygon, PolyWeight},
        via::ViaWeight,
        CompoundWeight,
    },
    math::Circle,
};

#[derive(Clone, Copy, Debug)]
pub enum TerminatingScheme {
    ExistingFixedDot(FixedDotIndex),
    Anteroute,
}

#[derive(Clone, Debug)]
pub struct AnterouterPlan {
    pub layer_map: BTreeMap<RatlineIndex, usize>,
    pub ratline_endpoint_dot_to_terminating_scheme: BTreeMap<FixedDotIndex, TerminatingScheme>,
}

pub struct Anterouter {
    plan: AnterouterPlan,
}

impl Anterouter {
    pub fn new(plan: AnterouterPlan) -> Self {
        Self { plan }
    }

    pub fn anteroute(&mut self, autorouter: &mut Autorouter<impl AccessMesadata>) {
        // PERF: Unnecessary clone.
        for (ratline, layer) in self.plan.layer_map.clone().iter() {
            let endpoint_indices = ratline.ref_(autorouter).endpoint_indices();
            let endpoint_dots = ratline.ref_(autorouter).endpoint_dots();

            autorouter
                .ratsnest
                .assign_layer_to_ratline(*ratline, *layer);

            if let Some(terminating_scheme) = self
                .plan
                .ratline_endpoint_dot_to_terminating_scheme
                .get(&endpoint_dots.0)
            {
                match terminating_scheme {
                    TerminatingScheme::ExistingFixedDot(terminating_dot) => {
                        autorouter.ratsnest.assign_terminating_dot_to_ratvertex(
                            endpoint_indices.0,
                            *layer,
                            *terminating_dot,
                        )
                    }
                    TerminatingScheme::Anteroute => {
                        self.anteroute_dot(autorouter, endpoint_indices.0, endpoint_dots.0, *layer)
                    }
                }
            }

            if let Some(terminating_scheme) = self
                .plan
                .ratline_endpoint_dot_to_terminating_scheme
                .get(&endpoint_dots.1)
            {
                match terminating_scheme {
                    TerminatingScheme::ExistingFixedDot(terminating_dot) => {
                        autorouter.ratsnest.assign_terminating_dot_to_ratvertex(
                            endpoint_indices.1,
                            *layer,
                            *terminating_dot,
                        )
                    }
                    TerminatingScheme::Anteroute => {
                        self.anteroute_dot(autorouter, endpoint_indices.1, endpoint_dots.1, *layer)
                    }
                }
            }
        }
    }

    fn anteroute_dot(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        to_layer: usize,
    ) {
        self.place_assignment_via_on_anchor(autorouter, ratvertex, dot, to_layer, [-20.0, 0.0])
    }

    fn anteroute_dot_to_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        target_layer: usize,
        anchor: [f64; 2],
    ) {
        self.place_assignment_via_on_anchor(autorouter, ratvertex, dot, target_layer, anchor)
    }

    fn place_assignment_via_on_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        target_layer: usize,
        anchor: [f64; 2],
    ) {
        let source_layer = autorouter.board().layout().drawing().primitive(dot).layer();

        // TODO: refactor code to remove this let statement.
        let PrimitiveShape::Dot(source_shape) =
            autorouter.board().layout().drawing().primitive(dot).shape()
        else {
            unreachable!();
        };

        let mut board_edit = BoardEdit::new();

        if let Ok((.., dots)) = autorouter.board.add_via(
            &mut board_edit,
            ViaWeight {
                from_layer: std::cmp::min(source_layer, target_layer),
                to_layer: std::cmp::max(source_layer, target_layer),
                circle: Circle {
                    pos: point! {
                        x: source_shape.center().x() + anchor[0] * source_shape.circle.r,
                        y: source_shape.center().y() + anchor[1] * source_shape.circle.r
                    },
                    r: 100.0,
                },
                maybe_net: autorouter
                    .board()
                    .layout()
                    .drawing()
                    .primitive(dot)
                    .maybe_net(),
            },
            autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(dot.into()))
                .cloned(),
        ) {
            let terminating_dot = dots
                .iter()
                .find(|dot| {
                    target_layer
                        == dot
                            .primitive_ref(autorouter.board().layout().drawing())
                            .layer()
                })
                .unwrap();
            autorouter.ratsnest.assign_terminating_dot_to_ratvertex(
                ratvertex,
                target_layer,
                *terminating_dot,
            );
        }
    }
}
