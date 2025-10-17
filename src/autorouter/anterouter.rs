// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use geo::{point, Distance, Euclidean, Point};
use petgraph::graph::NodeIndex;
use rstar::{Envelope, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        compass_direction::{CardinalDirection, CompassDirection, OrdinalDirection},
        ratline::RatlineUid,
        Autorouter,
    },
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct AnterouterOptions {
    pub fanout_clearance: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum TerminatingScheme {
    ExistingFixedDot(FixedDotIndex),
    Fanout,
}

#[derive(Clone, Debug)]
pub struct AnterouterPlan {
    pub layer_map: BTreeMap<RatlineUid, usize>,
    pub ratline_endpoint_dot_to_terminating_scheme: BTreeMap<FixedDotIndex, TerminatingScheme>,
}

pub struct Anterouter {
    plan: AnterouterPlan,
}

impl Anterouter {
    pub fn new(plan: AnterouterPlan) -> Self {
        Self { plan }
    }

    pub fn anteroute(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        recorder: &mut BoardEdit,
        options: &AnterouterOptions,
    ) {
        // PERF: Unnecessary clone.
        for (ratline, layer) in self.plan.layer_map.clone().iter() {
            let endpoint_indices = ratline.ref_(autorouter).endpoint_indices();
            let endpoint_dots = ratline.ref_(autorouter).endpoint_dots();

            autorouter
                .ratsnests
                .on_principal_layer_mut(ratline.principal_layer)
                .assign_layer_to_ratline(ratline.index, *layer);

            if let Some(terminating_scheme) = self
                .plan
                .ratline_endpoint_dot_to_terminating_scheme
                .get(&endpoint_dots.0)
            {
                match terminating_scheme {
                    TerminatingScheme::ExistingFixedDot(terminating_dot) => autorouter
                        .ratsnests
                        .on_principal_layer_mut(0)
                        .assign_terminating_dot_to_ratvertex(
                            endpoint_indices.0,
                            *layer,
                            *terminating_dot,
                        ),
                    TerminatingScheme::Fanout => self.anteroute_fanout(
                        autorouter,
                        recorder,
                        endpoint_indices.0,
                        *ratline,
                        endpoint_dots.0,
                        *layer,
                        options,
                    ),
                }
            }

            if let Some(terminating_scheme) = self
                .plan
                .ratline_endpoint_dot_to_terminating_scheme
                .get(&endpoint_dots.1)
            {
                match terminating_scheme {
                    TerminatingScheme::ExistingFixedDot(terminating_dot) => autorouter
                        .ratsnests
                        .on_principal_layer_mut(0)
                        .assign_terminating_dot_to_ratvertex(
                            endpoint_indices.1,
                            *layer,
                            *terminating_dot,
                        ),
                    TerminatingScheme::Fanout => self.anteroute_fanout(
                        autorouter,
                        recorder,
                        endpoint_indices.1,
                        *ratline,
                        endpoint_dots.1,
                        *layer,
                        options,
                    ),
                }
            }
        }
    }

    fn anteroute_fanout(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        recorder: &mut BoardEdit,
        ratvertex: NodeIndex<usize>,
        ratline: RatlineUid,
        source_dot: FixedDotIndex,
        target_layer: usize,
        options: &AnterouterOptions,
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
                recorder,
                ratvertex,
                source_dot,
                small_bbox,
                target_layer,
                CardinalDirection::nearest_from_vector(ratline_delta),
                options,
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
                recorder,
                ratvertex,
                source_dot,
                large_bbox,
                target_layer,
                CardinalDirection::nearest_from_vector(ratline_delta),
                options,
            )
            .is_ok()
        {
            return;
        }

        if self
            .anteroute_fanout_on_bbox(
                autorouter,
                recorder,
                ratvertex,
                source_dot,
                small_bbox,
                target_layer,
                OrdinalDirection::nearest_from_vector(ratline_delta),
                options,
            )
            .is_ok()
        {
            return;
        }

        if self
            .anteroute_fanout_on_bbox(
                autorouter,
                recorder,
                ratvertex,
                source_dot,
                large_bbox,
                target_layer,
                OrdinalDirection::nearest_from_vector(ratline_delta),
                options,
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
        recorder: &mut BoardEdit,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        preferred_compass_direction: impl CompassDirection,
        options: &AnterouterOptions,
    ) -> Result<(), ()> {
        if self
            .anteroute_fanout_on_bbox_in_direction(
                autorouter,
                recorder,
                ratvertex,
                source_dot,
                bbox,
                target_layer,
                preferred_compass_direction,
                options,
            )
            .is_ok()
        {
            return Ok(());
        }

        let mut counterclockwise_turning_cardinal_direction = preferred_compass_direction;
        let mut clockwise_turning_cardinal_direction = preferred_compass_direction;

        loop {
            counterclockwise_turning_cardinal_direction =
                counterclockwise_turning_cardinal_direction.turn_counterclockwise();

            if self
                .anteroute_fanout_on_bbox_in_direction(
                    autorouter,
                    recorder,
                    ratvertex,
                    source_dot,
                    bbox,
                    target_layer,
                    counterclockwise_turning_cardinal_direction,
                    options,
                )
                .is_ok()
            {
                return Ok(());
            }

            clockwise_turning_cardinal_direction =
                clockwise_turning_cardinal_direction.turn_clockwise();

            if self
                .anteroute_fanout_on_bbox_in_direction(
                    autorouter,
                    recorder,
                    ratvertex,
                    source_dot,
                    bbox,
                    target_layer,
                    clockwise_turning_cardinal_direction,
                    options,
                )
                .is_ok()
            {
                return Ok(());
            }

            if counterclockwise_turning_cardinal_direction == preferred_compass_direction
                || clockwise_turning_cardinal_direction == preferred_compass_direction
            {
                return Err(());
            }
        }
    }

    fn anteroute_fanout_on_bbox_in_direction(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        recorder: &mut BoardEdit,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        direction: impl Into<Point>,
        options: &AnterouterOptions,
    ) -> Result<(), ()> {
        let (via, dots) = self.place_fanout_via_on_bbox_in_direction(
            autorouter,
            recorder,
            ratvertex,
            source_dot,
            bbox,
            target_layer,
            direction,
            options,
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

        if let Ok(_) = autorouter.board.layout_mut().add_fixed_seg(
            &mut recorder.layout_edit,
            source_dot,
            fanout_dot,
            FixedSegWeight(GeneralSegWeight {
                width: 100.0,
                layer,
                maybe_net,
            }),
        ) {
            Ok(())
        } else {
            autorouter.board.remove_via(recorder, via, dots);
            Err(())
        }
    }

    fn place_fanout_via_on_bbox_in_direction(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        recorder: &mut BoardEdit,
        ratvertex: NodeIndex<usize>,
        source_dot: FixedDotIndex,
        bbox: AABB<[f64; 2]>,
        target_layer: usize,
        direction: impl Into<Point>,
        options: &AnterouterOptions,
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
        let bbox_center = point! {x: bbox.center()[0], y: bbox.center()[1]};
        let center = autorouter
            .board()
            .layout()
            .drawing()
            .primitive(source_dot)
            .shape()
            .center();

        let cardinal_direction_vector = direction.into();

        let bbox_anchor = point! {
            x: (bbox.upper()[0] - bbox.lower()[0]) / 2.0 * cardinal_direction_vector.x(),
            y: (bbox.upper()[1] - bbox.lower()[1]) / 2.0 * cardinal_direction_vector.y(),
        };
        let bbox_anchor_with_clearance = bbox_anchor
            * (1.0
                + options.fanout_clearance
                    / Euclidean::distance(bbox_anchor, point! {x: 0.0, y: 0.0}));
        let pos = center
            + cardinal_direction_vector
                * (bbox_center + bbox_anchor_with_clearance - center)
                    .dot(cardinal_direction_vector);

        if let Ok((via, dots)) = autorouter.board.add_via(
            recorder,
            ViaWeight {
                from_layer: std::cmp::min(source_layer, target_layer),
                to_layer: std::cmp::max(source_layer, target_layer),
                circle: Circle { pos, r: 100.0 },
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
            autorouter
                .ratsnests
                .on_principal_layer_mut(0)
                .assign_terminating_dot_to_ratvertex(ratvertex, target_layer, *terminating_dot);
            Ok((via, dots))
        } else {
            Err(())
        }
    }
}
