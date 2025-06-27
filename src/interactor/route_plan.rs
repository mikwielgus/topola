// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::ops::ControlFlow;
use geo::{LineString, Point};
use rstar::AABB;

use crate::{
    autorouter::invoker::Invoker,
    board::AccessMesadata,
    drawing::dot::FixedDotIndex,
    geometry::shape::AccessShape as _,
    stepper::{Abort, OnEvent, Step},
};

use super::{
    activity::{ActivityContext, InteractiveEvent, InteractiveEventKind},
    interaction::InteractionError,
};

/// An interactive collector for a route
#[derive(Debug)]
pub struct RoutePlan {
    pub active_layer: usize,
    pub start_pin: Option<(FixedDotIndex, Point)>,
    pub end_pin: Option<(FixedDotIndex, Point)>,
    pub lines: LineString,
}

fn try_select_pin<M: AccessMesadata>(
    context: &ActivityContext<M>,
) -> Option<(usize, FixedDotIndex, Point)> {
    let active_layer = context.interactive_input.active_layer?;
    let pos = context.interactive_input.pointer_pos;
    let board = context.invoker.autorouter().board();
    let layout = board.layout();
    let bbox = AABB::from_point([pos.x(), pos.y()]);

    // gather relevant resolved selectors
    let mut pin = None;
    let mut apex_node = None;
    for &geom in
        layout
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [pos.x(), pos.y(), active_layer as f64 - f64::EPSILON],
                [pos.x(), pos.y(), active_layer as f64 + f64::EPSILON],
            ))
    {
        let node = geom.data;
        if layout.node_shape(node).intersects_with_bbox(&bbox) {
            if let Some(pin_name) = board.node_pinname(&node) {
                // insert and check that we selected exactly one pin / node
                log::debug!("node {:?} -> pin {:?}", node, pin_name);
                if let Some(old_pin) = pin {
                    if pin_name != old_pin {
                        log::debug!(
                            "rejected pin due to ambiguity: [{:?}, {:?}]",
                            old_pin,
                            pin_name
                        );
                        return None;
                    }
                    continue;
                }
                pin = Some(pin_name);
                let (idx, pos) = board.pin_apex(pin_name, active_layer).unwrap();
                if let Some((idx2, _)) = apex_node {
                    if idx != idx2 {
                        // node index is ambiguous
                        log::debug!("rejected pin due to ambiguity: [{:?}, {:?}]", idx2, idx);
                        return None;
                    }
                } else {
                    apex_node = Some((idx, pos));
                }
            }
        }
    }

    let ret = apex_node.map(|(idx, pos)| (active_layer, idx, pos));
    log::debug!("result {:?}", ret);
    ret
}

impl RoutePlan {
    pub fn new(active_layer: usize) -> Self {
        log::debug!("initialized RoutePlan");
        Self {
            active_layer,
            start_pin: None,
            end_pin: None,
            lines: LineString(Vec::new()),
        }
    }

    pub fn last_pos(&self) -> Option<Point> {
        self.lines.0.last().map(|i| Point(*i))
    }
}

impl<M: AccessMesadata> Step<ActivityContext<'_, M>, String> for RoutePlan {
    type Error = InteractionError;

    fn step(
        &mut self,
        _context: &mut ActivityContext<M>,
    ) -> Result<ControlFlow<String>, InteractionError> {
        Ok(ControlFlow::Continue(()))
    }
}

impl<M: AccessMesadata> Abort<Invoker<M>> for RoutePlan {
    fn abort(&mut self, _context: &mut Invoker<M>) {
        self.start_pin = None;
        self.end_pin = None;
        self.lines.0.clear();
    }
}

impl<M: AccessMesadata> OnEvent<ActivityContext<'_, M>, InteractiveEvent> for RoutePlan {
    type Output = Result<(), InteractionError>;

    fn on_event(
        &mut self,
        context: &mut ActivityContext<M>,
        event: InteractiveEvent,
    ) -> Result<(), InteractionError> {
        match event.kind {
            InteractiveEventKind::PointerPrimaryButtonClicked if self.end_pin.is_none() => {
                if let Some((layer, idx, pos)) = try_select_pin(context) {
                    // make sure double-click or such doesn't corrupt state
                    if let Some(start_pin) = self.start_pin {
                        if self.active_layer == layer && idx != start_pin.0 {
                            log::debug!("clicked on end pin: {:?}", idx);
                            self.lines.0.push(pos.0);
                            self.end_pin = Some((idx, pos));
                        }
                    } else {
                        log::debug!("clicked on start pin: {:?}", idx);
                        self.active_layer = layer;
                        self.start_pin = Some((idx, pos));
                        self.lines.0.push(pos.0);
                    }
                } else if let Some(last_pos) = self.last_pos() {
                    let pos = context.interactive_input.pointer_pos;
                    if last_pos != pos {
                        self.lines.0.push(pos.0);
                    }
                }
            }
            InteractiveEventKind::PointerSecondaryButtonClicked => {
                if self.end_pin.is_some() {
                    log::debug!("un-clicked end pin");
                    self.end_pin = None;
                    self.lines.0.pop();
                }
                if !self.lines.0.is_empty() {
                    self.lines.0.pop();
                    if self.lines.0.is_empty() {
                        log::debug!("un-clicked start pin");
                        self.start_pin = None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
