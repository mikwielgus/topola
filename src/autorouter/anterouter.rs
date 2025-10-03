// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use geo::{point, Point};
use petgraph::graph::{EdgeIndex, NodeIndex};
use rstar::{Envelope, RTreeObject, AABB};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{compass_direction::CardinalDirection, ratline::RatlineIndex, Autorouter},
    board::edit::BoardEdit,
    drawing::{
        dot::FixedDotIndex,
        graph::{GetMaybeNet, MakePrimitiveRef},
        primitive::MakePrimitiveShape,
    },
    geometry::{GenericNode, GetLayer},
    graph::MakeRef,
    layout::{poly::MakePolygon, via::ViaWeight},
    math::Circle,
};

#[derive(Clone, Copy, Debug)]
pub enum TerminatingScheme {
    ExistingFixedDot(FixedDotIndex),
    Fanout,
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
                    TerminatingScheme::Fanout => self.anteroute_fanout(
                        autorouter,
                        endpoint_indices.0,
                        *ratline,
                        endpoint_dots.0,
                        *layer,
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
                    TerminatingScheme::Fanout => self.anteroute_fanout(
                        autorouter,
                        endpoint_indices.1,
                        *ratline,
                        endpoint_dots.1,
                        *layer,
                    ),
                }
            }
        }
    }

    fn anteroute_fanout(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        ratline: EdgeIndex<usize>,
        source_dot: FixedDotIndex,
        target_layer: usize,
    ) {
        let mut ratline_delta: Point = ratline.ref_(autorouter).line_segment().delta().into();

        if ratvertex == ratline.ref_(autorouter).endpoint_indices().1 {
            ratline_delta = -ratline_delta;
        }

        let cardinal_direction = CardinalDirection::nearest_to_vector(ratline_delta);

        if self
            .anteroute_fanout_to_anchor(
                autorouter,
                ratvertex,
                source_dot,
                target_layer,
                Point::from(cardinal_direction) * 1.4,
            )
            .is_ok()
        {
            return;
        }

        let mut counterclockwise_turning = cardinal_direction;
        let mut clockwise_turning = cardinal_direction;

        loop {
            counterclockwise_turning = counterclockwise_turning.turn_counterclockwise();

            if self
                .anteroute_fanout_to_anchor(
                    autorouter,
                    ratvertex,
                    source_dot,
                    target_layer,
                    Point::from(counterclockwise_turning) * 1.4,
                )
                .is_ok()
            {
                return;
            }

            clockwise_turning = clockwise_turning.turn_clockwise();

            if self
                .anteroute_fanout_to_anchor(
                    autorouter,
                    ratvertex,
                    source_dot,
                    target_layer,
                    Point::from(clockwise_turning) * 1.4,
                )
                .is_ok()
            {
                return;
            }

            if counterclockwise_turning == cardinal_direction
                || clockwise_turning == cardinal_direction
            {
                break;
                //panic!();
            }
        }
    }

    fn anteroute_fanout_to_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        target_layer: usize,
        bbox_to_anchor: Point,
    ) -> Result<(), ()> {
        self.place_fanout_via_on_anchor(
            autorouter,
            ratvertex,
            source_dot,
            target_layer,
            bbox_to_anchor,
        )
    }

    fn place_fanout_via_on_anchor(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        dot: FixedDotIndex,
        target_layer: usize,
        endpoint_dot_bbox_to_anchor: Point,
    ) -> Result<(), ()> {
        let source_layer = autorouter.board().layout().drawing().primitive(dot).layer();
        let pin_maybe_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(dot)
            .maybe_net();

        let pin_bbox = if let Some(poly) = autorouter.board().layout().primitive_poly(dot.into()) {
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
            x: pin_bbox.center()[0] + (pin_bbox.upper()[0] - pin_bbox.lower()[0]) / 2.0 * endpoint_dot_bbox_to_anchor.x(),
            y: pin_bbox.center()[1] + (pin_bbox.upper()[1] - pin_bbox.lower()[1]) / 2.0 * endpoint_dot_bbox_to_anchor.y(),
        };

        //let via_bbox_to_anchor = [-pin_bbox_to_anchor[0], -pin_bbox_to_anchor[1]];

        let mut board_edit = BoardEdit::new();

        if let Ok((.., dots)) = autorouter.board.add_via(
            &mut board_edit,
            ViaWeight {
                from_layer: std::cmp::min(source_layer, target_layer),
                to_layer: std::cmp::max(source_layer, target_layer),
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
            Ok(())
        } else {
            Err(())
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
