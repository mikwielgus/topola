// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages autorouting of ratlines in a layout, tracking status and processed
//! routing steps.

use std::ops::ControlFlow;

use crate::{
    board::AccessMesadata,
    drawing::{band::BandTermsegIndex, graph::PrimitiveIndex, Collect},
    geometry::{edit::ApplyGeometryEdit, primitive::PrimitiveShape},
    graph::MakeRef,
    layout::LayoutEdit,
    router::{
        navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper, RouteStepper, Router,
    },
    stepper::{Abort, EstimateProgress, Step},
};

use super::{
    invoker::GetDebugOverlayData, ratline::RatlineIndex, Autorouter, AutorouterError,
    AutorouterOptions,
};

/// Represents the current status of the autoroute operation.
pub enum AutorouteContinueStatus {
    /// The autoroute is currently running and in progress.
    Running,
    /// A specific segment has been successfully routed.
    Routed(BandTermsegIndex),
    /// A specific segment was already routed and has been skipped.
    Skipped(BandTermsegIndex),
}

/// Manages the autorouting process across multiple ratlines.
pub struct AutorouteExecutionStepper {
    /// The ratlines which we are routing.
    ratlines: Vec<RatlineIndex>,
    /// Keeps track of the current ratline being routed, if one is active.
    curr_ratline_index: usize,
    /// Stores the current route being processed, if any.
    route: Option<RouteStepper>,
    /// The options for the autorouting process, defining how routing should be carried out.
    options: AutorouterOptions,
}

impl AutorouteExecutionStepper {
    /// Initializes a new [`AutorouteExecutionStepper`] instance.
    ///
    /// This method sets up the routing process by accepting the execution properties.
    /// It prepares the first ratline to route
    /// and stores the associated data for future routing steps.
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        if ratlines.is_empty() {
            return Err(AutorouterError::NothingToRoute);
        };

        let (origin, destination) = ratlines[0].ref_(autorouter).endpoint_dots();
        let mut router = Router::new(autorouter.board.layout_mut(), options.router_options);

        Ok(Self {
            ratlines,
            curr_ratline_index: 0,
            route: Some(router.route(
                LayoutEdit::new(),
                origin,
                destination,
                options.router_options.routed_band_width,
            )?),
            options,
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
        if self.curr_ratline_index >= self.ratlines.len() {
            let recorder = if let Some(taken_route) = self.route.take() {
                let (_thetastar, navcord, ..) = taken_route.dissolve();
                navcord.recorder
            } else {
                LayoutEdit::new()
            };

            return Ok(ControlFlow::Break(Some(recorder)));
        }

        let Some(ref mut route) = self.route else {
            // Shouldn't happen.
            return Ok(ControlFlow::Break(None));
        };

        let (source, target) = self.ratlines[self.curr_ratline_index]
            .ref_(autorouter)
            .endpoint_dots();

        let ret = if let Some(band_termseg) = autorouter.board.band_between_nodes(source, target) {
            AutorouteContinueStatus::Skipped(band_termseg[false])
        } else {
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
                .loose_band_uid(band_termseg.into())
                .expect("a completely routed band should've Seg's as ends");

            autorouter.ratsnest.assign_band_termseg_to_ratline(
                self.ratlines[self.curr_ratline_index],
                band_termseg,
            );

            autorouter
                .board
                .try_set_band_between_nodes(source, target, band);

            AutorouteContinueStatus::Routed(band_termseg)
        };

        self.curr_ratline_index += 1;

        if let Some(new_ratline) = self.ratlines.get(self.curr_ratline_index) {
            let (source, target) = new_ratline.ref_(autorouter).endpoint_dots();
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

            let recorder = if let Some(taken_route) = self.route.take() {
                let (_thetastar, navcord, ..) = taken_route.dissolve();
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
        }

        Ok(ControlFlow::Continue(ret))
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for AutorouteExecutionStepper {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        if let Some(ref route) = self.route {
            autorouter.board.apply(&route.navcord().recorder.reverse());
            self.curr_ratline_index = self.ratlines.len();
        }
    }
}

impl EstimateProgress for AutorouteExecutionStepper {
    type Value = f64;

    fn estimate_progress_value(&self) -> f64 {
        self.curr_ratline_index as f64
            + self.route.as_ref().map_or(0.0, |route| {
                route.estimate_progress_value() / route.estimate_progress_maximum()
            })
    }

    fn estimate_progress_maximum(&self) -> f64 {
        self.ratlines.len() as f64
    }
}

impl GetDebugOverlayData for AutorouteExecutionStepper {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.route.as_ref().map(|route| route.thetastar())
    }

    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.route.as_ref().map(|route| route.navcord())
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        self.route.as_ref().map_or(&[], |route| route.ghosts())
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.route.as_ref().map_or(&[], |route| route.obstacles())
    }
}
