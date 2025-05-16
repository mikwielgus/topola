// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages autorouting of ratlines in a layout, tracking status and processed
//! routing steps.

use std::ops::ControlFlow;

use petgraph::graph::EdgeIndex;

use crate::{
    board::AccessMesadata,
    drawing::{band::BandTermsegIndex, graph::PrimitiveIndex, Collect},
    geometry::primitive::PrimitiveShape,
    layout::LayoutEdit,
    router::{
        astar::AstarStepper,
        navcord::Navcord,
        navmesh::{Navmesh, NavvertexIndex},
        RouteStepper, Router,
    },
    stepper::Step,
};

use super::{
    invoker::{
        GetGhosts, GetMaybeAstarStepper, GetMaybeNavcord, GetNavmeshDebugTexts, GetObstacles,
    },
    Autorouter, AutorouterError, AutorouterOptions,
};

/// Represents the current status of the autoroute operation.
pub enum AutorouteContinueStatus {
    /// The autoroute is currently running and in progress.
    Running,
    /// A specific segment has been successfully routed.
    Routed(BandTermsegIndex),
}

/// Manages the autorouting process across multiple ratlines.
pub struct AutorouteExecutionStepper {
    /// An iterator over ratlines that tracks which segments still need to be routed.
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    /// The options for the autorouting process, defining how routing should be carried out.
    options: AutorouterOptions,
    /// Stores the current route being processed, if any.
    route: Option<RouteStepper>,
    /// Keeps track of the current ratline being routed, if one is active.
    curr_ratline: Option<EdgeIndex<usize>>,
}

impl AutorouteExecutionStepper {
    /// Initializes a new [`AutorouteExecutionStepper`] instance.
    ///
    /// This method sets up the routing process by accepting the execution properties.
    /// It prepares the first ratline to route
    /// and stores the associated data for future routing steps.
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(curr_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (origin, destination) = autorouter.ratline_endpoints(curr_ratline);
        let mut router = Router::new(autorouter.board.layout_mut(), options.router_options);

        Ok(Self {
            ratlines_iter,
            options,
            route: Some(router.route(
                LayoutEdit::new(),
                origin,
                destination,
                options.router_options.routed_band_width,
            )?),
            curr_ratline: Some(curr_ratline),
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<LayoutEdit>, AutorouteContinueStatus>
    for AutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<LayoutEdit>, AutorouteContinueStatus>, AutorouterError> {
        let Some(curr_ratline) = self.curr_ratline else {
            let recorder = if let Some(taken_route) = self.route.take() {
                let (_astar, navcord, ..) = taken_route.dissolve();
                navcord.recorder
            } else {
                LayoutEdit::new()
            };

            return Ok(ControlFlow::Break(Some(recorder)));
        };

        let Some(ref mut route) = self.route else {
            // Shouldn't happen.
            return Ok(ControlFlow::Break(None));
        };

        let (source, target) = autorouter.ratline_endpoints(curr_ratline);

        let band_termseg = {
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

            let ControlFlow::Break(band_termseg) = route.step(&mut router)? else {
                return Ok(ControlFlow::Continue(AutorouteContinueStatus::Running));
            };
            band_termseg
        };

        let band = autorouter
            .board
            .layout()
            .drawing()
            .loose_band_uid(band_termseg.into());

        autorouter
            .ratsnest
            .assign_band_termseg_to_ratline(self.curr_ratline.unwrap(), band_termseg);

        autorouter
            .board
            .try_set_band_between_nodes(source, target, band);

        if let Some(new_ratline) = self.ratlines_iter.next() {
            let (source, target) = autorouter.ratline_endpoints(new_ratline);
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

            self.curr_ratline = Some(new_ratline);

            let recorder = if let Some(taken_route) = self.route.take() {
                let (_astar, navcord, ..) = taken_route.dissolve();
                navcord.recorder
            } else {
                LayoutEdit::new()
            };

            self.route = Some(router.route(
                recorder,
                source,
                target,
                self.options.router_options.routed_band_width,
            )?);
        } else {
            self.curr_ratline = None;
        }

        Ok(ControlFlow::Continue(AutorouteContinueStatus::Routed(
            band_termseg,
        )))
    }
}

impl GetMaybeAstarStepper for AutorouteExecutionStepper {
    fn maybe_astar(&self) -> Option<&AstarStepper<Navmesh, f64>> {
        self.route.as_ref().map(|route| route.astar())
    }
}

impl GetMaybeNavcord for AutorouteExecutionStepper {
    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.route.as_ref().map(|route| route.navcord())
    }
}

impl GetGhosts for AutorouteExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.route.as_ref().map_or(&[], |route| route.ghosts())
    }
}

impl GetObstacles for AutorouteExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.route.as_ref().map_or(&[], |route| route.obstacles())
    }
}

impl GetNavmeshDebugTexts for AutorouteExecutionStepper {}
