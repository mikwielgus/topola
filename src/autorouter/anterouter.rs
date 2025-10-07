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
        seg::{FixedSegWeight, GeneralSegWeight},
    },
    geometry::{shape::AccessShape, GenericNode, GetLayer},
    graph::{GenericIndex, MakeRef},
    layout::{poly::MakePolygon, via::ViaWeight, LayoutEdit},
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

        let small_bbox = if let Some(poly) = autorouter
            .board()
            .layout()
            .primitive_poly(source_dot.into())
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
                .primitive(source_dot)
                .shape()
                .envelope()
        };

        if self
            .anteroute_fanout_on_bbox(
                autorouter,
                ratvertex,
                source_dot,
                small_bbox,
                target_layer,
                CardinalDirection::nearest_to_vector(ratline_delta),
            )
            .is_ok()
        {
            return;
        }

        let mut large_bbox = small_bbox;

        for enclosing_poly in autorouter.board().layout().polys_enclosing_point(
            source_dot
                .primitive_ref(autorouter.board().layout().drawing())
                .shape()
                .center(),
        ) {
            let enclosing_bbox = enclosing_poly
                .ref_(autorouter.board().layout())
                .shape()
                .envelope();
            large_bbox.merge(&AABB::<[f64; 2]>::from_corners(
                [enclosing_bbox.lower().x(), enclosing_bbox.lower().y()],
                [enclosing_bbox.upper().x(), enclosing_bbox.upper().y()],
            ));
        }

        if self
            .anteroute_fanout_on_bbox(
                autorouter,
                ratvertex,
                source_dot,
                large_bbox,
                target_layer,
                CardinalDirection::nearest_to_vector(ratline_delta),
            )
            .is_ok()
        {
            return;
        }

        panic!();
    }

    fn anteroute_fanout_on_bbox(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        preferred_cardinal_direction: CardinalDirection,
    ) -> Result<(), ()> {
        if self
            .anteroute_fanout_on_bbox_in_cardinal_direction(
                autorouter,
                ratvertex,
                source_dot,
                bbox,
                target_layer,
                preferred_cardinal_direction,
            )
            .is_ok()
        {
            return Ok(());
        }

        let mut counterclockwise_turning_cardinal_direction = preferred_cardinal_direction;
        let mut clockwise_turning_cardinal_direction = preferred_cardinal_direction;

        loop {
            counterclockwise_turning_cardinal_direction =
                counterclockwise_turning_cardinal_direction.turn_counterclockwise();

            if self
                .anteroute_fanout_on_bbox_in_cardinal_direction(
                    autorouter,
                    ratvertex,
                    source_dot,
                    bbox,
                    target_layer,
                    counterclockwise_turning_cardinal_direction,
                )
                .is_ok()
            {
                return Ok(());
            }

            clockwise_turning_cardinal_direction =
                clockwise_turning_cardinal_direction.turn_clockwise();

            if self
                .anteroute_fanout_on_bbox_in_cardinal_direction(
                    autorouter,
                    ratvertex,
                    source_dot,
                    bbox,
                    target_layer,
                    clockwise_turning_cardinal_direction,
                )
                .is_ok()
            {
                return Ok(());
            }

            if counterclockwise_turning_cardinal_direction == preferred_cardinal_direction
                || clockwise_turning_cardinal_direction == preferred_cardinal_direction
            {
                return Err(());
            }
        }
    }

    fn anteroute_fanout_on_bbox_in_cardinal_direction(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        cardinal_direction: CardinalDirection,
    ) -> Result<(), ()> {
        let (_, dots) = self.place_fanout_via_on_bbox_in_cardinal_direction(
            autorouter,
            ratvertex,
            source_dot,
            bbox,
            target_layer,
            cardinal_direction,
        )?;

        let layer = source_dot
            .primitive_ref(autorouter.board().layout().drawing())
            .layer();
        let maybe_net = source_dot
            .primitive_ref(autorouter.board().layout().drawing())
            .maybe_net();
        let fanout_dot = *dots
            .iter()
            .find(|dot| {
                source_dot
                    .primitive_ref(autorouter.board().layout().drawing())
                    .layer()
                    == dot
                        .primitive_ref(autorouter.board().layout().drawing())
                        .layer()
            })
            .unwrap();

        autorouter.board.layout_mut().add_fixed_seg(
            &mut LayoutEdit::new(),
            source_dot,
            fanout_dot,
            FixedSegWeight(GeneralSegWeight {
                width: 100.0,
                layer,
                maybe_net,
            }),
        );

        Ok(())
    }

    fn place_fanout_via_on_bbox_in_cardinal_direction(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        cardinal_direction: CardinalDirection,
    ) -> Result<(GenericIndex<ViaWeight>, Vec<FixedDotIndex>), ()> {
        let source_layer = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .layer();
        let pin_maybe_net = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .maybe_net();
        let center = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .shape()
            .center();

        let bbox_to_anchor = Point::from(cardinal_direction) * 1.4;
        let bbox_anchor = point! {
            x: center.x() + (bbox.upper()[0] - bbox.lower()[0]) / 2.0 * bbox_to_anchor.x(),
            y: center.y() + (bbox.upper()[1] - bbox.lower()[1]) / 2.0 * bbox_to_anchor.y(),
        };

        //let via_bbox_to_anchor = [-pin_bbox_to_anchor[0], -pin_bbox_to_anchor[1]];

        let mut board_edit = BoardEdit::new();

        if let Ok((via, dots)) = autorouter.board.add_via(
            &mut board_edit,
            ViaWeight {
                from_layer: std::cmp::min(source_layer, target_layer),
                to_layer: std::cmp::max(source_layer, target_layer),
                circle: Circle {
                    pos: bbox_anchor,
                    r: 100.0,
                },
                maybe_net: pin_maybe_net,
            },
            autorouter
                .board()
                .node_pinname(&GenericNode::Primitive(source_dot.into()))
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
            Ok((via, dots))
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
