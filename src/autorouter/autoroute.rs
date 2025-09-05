// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages autorouting of ratlines in a layout, tracking status and processed
//! routing steps.

use std::ops::ControlFlow;

use crate::{
    board::{
        edit::{BoardDataEdit, BoardEdit},
        AccessMesadata,
    },
    drawing::{band::BandTermsegIndex, graph::PrimitiveIndex},
    geometry::{edit::Edit, primitive::PrimitiveShape},
    graph::MakeRef,
    layout::LayoutEdit,
    router::{
        navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper, RouteStepper, Router,
    },
    stepper::{Abort, EstimateProgress, Permutate, Step},
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
    /// A specific segment had been already routed and has been skipped.
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
    /// Records the changes to the layout, one routed band per item.
    layout_edits: Vec<LayoutEdit>,
    /// Records the changes to the board data, one routed band per item.
    board_data_edits: Vec<BoardDataEdit>,
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
            layout_edits: vec![],
            board_data_edits: vec![],
            options,
        })
    }

    fn backtrace_to_index(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        index: usize,
    ) -> Result<(), AutorouterError> {
        if index >= self.board_data_edits.len() {
            return Err(AutorouterError::NothingToUndoForPermutation);
        }

        self.dissolve_route_stepper_and_push_layout_edit();

        let board_edit = BoardEdit::new_from_edits(
            BoardDataEdit::merge_iter(self.board_data_edits.split_off(index)),
            LayoutEdit::merge_iter(self.layout_edits.split_off(index)),
        );

        autorouter.board.apply_edit(&board_edit.reverse());

        let (origin, destination) = self.ratlines[index].ref_(autorouter).endpoint_dots();
        let mut router = Router::new(autorouter.board.layout_mut(), self.options.router_options);

        self.route = Some(router.route(
            LayoutEdit::new(),
            origin,
            destination,
            self.options.router_options.routed_band_width,
        )?);

        self.curr_ratline_index = index;
        Ok(())
    }

    fn dissolve_route_stepper_and_push_layout_edit(&mut self) {
        if let Some(taken_route) = self.route.take() {
            let (_thetastar, navcord, ..) = taken_route.dissolve();
            self.layout_edits.push(navcord.recorder);
        }
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, AutorouteContinueStatus>
    for AutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, AutorouteContinueStatus>, AutorouterError> {
        // TODO: Use a proper state machine here for better readability?

        if self.curr_ratline_index >= self.ratlines.len() {
            self.dissolve_route_stepper_and_push_layout_edit();

            return Ok(ControlFlow::Break(Some(BoardEdit::new_from_edits(
                BoardDataEdit::merge_iter(self.board_data_edits.iter().cloned()),
                // FIXME: This is a large clone. We probably need some
                // high-level, say `AutorouteEdit`, struct to store sequences
                // of edits.
                LayoutEdit::merge_iter(self.layout_edits.iter().cloned()),
            ))));
        }

        let Some(ref mut route) = self.route else {
            // May happen if stepper was aborted.
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
                .find_loose_band_uid(band_termseg.into())
                .expect("a completely routed band should've Seg's as ends");

            autorouter.ratsnest.assign_band_termseg_to_ratline(
                self.ratlines[self.curr_ratline_index],
                band_termseg,
            );

            let mut board_data_edit = BoardDataEdit::new();

            autorouter
                .board
                .try_set_band_between_nodes(&mut board_data_edit, source, target, band);

            self.board_data_edits.push(board_data_edit);

            AutorouteContinueStatus::Routed(band_termseg)
        };

        self.curr_ratline_index += 1;

        if let Some(new_ratline) = self.ratlines.get(self.curr_ratline_index) {
            let (source, target) = new_ratline.ref_(autorouter).endpoint_dots();
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

            self.dissolve_route_stepper_and_push_layout_edit();
            let recorder = LayoutEdit::new();

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
        self.backtrace_to_index(autorouter, 0);
        self.curr_ratline_index = self.ratlines.len();
    }
}

impl<M: AccessMesadata> Permutate<Autorouter<M>> for AutorouteExecutionStepper {
    type Index = RatlineIndex;
    type Output = Result<(), AutorouterError>;

    fn permutate(
        &mut self,
        autorouter: &mut Autorouter<M>,
        permutation: Vec<RatlineIndex>,
    ) -> Result<(), AutorouterError> {
        let Some(new_index) = permutation
            .iter()
            .zip(self.ratlines.iter())
            .position(|(permuted, original)| *permuted != *original)
        else {
            return Err(AutorouterError::NothingToUndoForPermutation);
        };
        self.ratlines = permutation;

        self.backtrace_to_index(autorouter, new_index)?;
        Ok(())
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
