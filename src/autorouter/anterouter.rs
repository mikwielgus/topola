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
    geometry::{GenericNode, GetLayer},
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
    Anteroute([f64; 2]),
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
                    TerminatingScheme::Anteroute(pin_bbox_to_anchor) => self
                        .anteroute_dot_to_anchor(
                            autorouter,
                            endpoint_indices.0,
                            endpoint_dots.0,
                            *layer,
                            *pin_bbox_to_anchor,
                        ),
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
                    TerminatingScheme::Anteroute(pin_bbox_to_anchor) => self
                        .anteroute_dot_to_anchor(
                            autorouter,
                            endpoint_indices.1,
                            endpoint_dots.1,
                            *layer,
                            *pin_bbox_to_anchor,
                        ),
                }
            }
        }
    }

    fn anteroute_dot_to_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        to_layer: usize,
        endpoint_dot_bbox_to_anchor: [f64; 2],
    ) {
        self.place_assignment_via_on_anchor(
            autorouter,
            ratvertex,
            dot,
            to_layer,
            endpoint_dot_bbox_to_anchor,
        )
    }

    fn place_assignment_via_on_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        to_layer: usize,
        endpoint_dot_bbox_to_anchor: [f64; 2],
    ) {
        let pin_layer = autorouter.board().layout().drawing().primitive(dot).layer();
        let pin_maybe_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(dot)
            .maybe_net();

        let pin_bbox = if let Some(poly) = autorouter
            .board()
            .layout()
            .drawing()
            .compounds(GenericIndex::<()>::new(dot.index()))
            .find_map(|(_, compound)| {
                if let CompoundWeight::Poly(_) = autorouter
                    .board()
                    .layout()
                    .drawing()
                    .compound_weight(compound)
                {
                    Some(compound)
                } else {
                    None
                }
            })
            .map(|compound| GenericIndex::<PolyWeight>::new(compound.index()))
        {
            let bbox = poly.ref_(autorouter.board().layout()).shape().envelope();
            AABB::<[f64; 2]>::from_corners(
                [bbox.lower().x(), bbox.lower().y()],
                [bbox.upper().x(), bbox.upper().y()],
            )
        } else {
            autorouter
                .board()
                .layout()
                .drawing()
                .primitive(dot)
                .shape()
                .envelope()
        };

        //let pin_bbox_anchor = pin_bbox.center() + (pin_bbox.upper() - pin_bbox.lower()) * pin_bbox_to_anchor;
        let pin_bbox_anchor = point! {
            x: pin_bbox.center()[0] + (pin_bbox.upper()[0] - pin_bbox.lower()[0]) / 2.0 * endpoint_dot_bbox_to_anchor[0],
            y: pin_bbox.center()[1] + (pin_bbox.upper()[1] - pin_bbox.lower()[1]) / 2.0 * endpoint_dot_bbox_to_anchor[1],
        };

        //let via_bbox_to_anchor = [-pin_bbox_to_anchor[0], -pin_bbox_to_anchor[1]];

        let mut board_edit = BoardEdit::new();

        if let Ok((.., dots)) = autorouter.board.add_via(
            &mut board_edit,
            ViaWeight {
                from_layer: std::cmp::min(pin_layer, to_layer),
                to_layer: std::cmp::max(pin_layer, to_layer),
                circle: Circle {
                    pos: pin_bbox_anchor,
                    r: 100.0,
                },
                maybe_net: pin_maybe_net,
            },
            autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(dot.into()))
                .cloned(),
        ) {
            let terminating_dot = dots
                .iter()
                .find(|dot| {
                    to_layer
                        == dot
                            .primitive_ref(autorouter.board().layout().drawing())
                            .layer()
                })
                .unwrap();
            autorouter.ratsnest.assign_terminating_dot_to_ratvertex(
                ratvertex,
                to_layer,
                *terminating_dot,
            );
        }
        /*let bbox = if let Some(poly) = autorouter.board().layout().drawing().geometry().compounds(dot).find(|(_, compound_weight)| {
            matches!(compound_weight, CompoundWeight::Poly(..))
        }) {
            poly.ref_(autorouter.board().layout()).polygon()
        } else {
            //
        }*/
    }
}
