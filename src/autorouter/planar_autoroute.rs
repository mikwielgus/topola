// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages autorouting of ratlines in a layout, tracking status and processed
//! routing steps.

use std::{collections::BTreeMap, ops::ControlFlow};

use derive_getters::Getters;

use crate::{
    board::{
        edit::{BoardDataEdit, BoardEdit},
        AccessMesadata,
    },
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, graph::PrimitiveIndex},
    geometry::{edit::Edit, primitive::PrimitiveShape},
    graph::MakeRef,
    layout::LayoutEdit,
    router::{
        navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper, RouteStepper, Router,
    },
    stepper::{Abort, EstimateProgress, Reconfigure, Step},
};

use super::{
    invoker::GetDebugOverlayData, ratline::RatlineUid, Autorouter, AutorouterError,
    PlanarAutorouteOptions,
};

#[derive(Clone, Debug)]
pub struct PlanarAutorouteConfiguration {
    pub ratlines: Vec<RatlineUid>,
    pub terminating_dot_map: BTreeMap<(RatlineUid, FixedDotIndex), FixedDotIndex>,
}

impl PlanarAutorouteConfiguration {
    pub fn ratline_terminating_dots(
        &self,
        autorouter: &Autorouter<impl AccessMesadata>,
        ratline_index: usize,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let ratline = self.ratlines[ratline_index];
        let endpoint_dots = ratline.ref_(autorouter).endpoint_dots();
        (
            *self
                .terminating_dot_map
                .get(&(ratline, endpoint_dots.0))
                .unwrap_or(&endpoint_dots.0),
            *self
                .terminating_dot_map
                .get(&(ratline, endpoint_dots.1))
                .unwrap_or(&endpoint_dots.1),
        )
    }
}

#[derive(Clone, Debug)]
pub struct PlanarAutorouteCosts {
    pub lengths: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct PlanarAutorouteConfigurationStatus {
    pub configuration: PlanarAutorouteConfiguration,
    pub costs: PlanarAutorouteCosts,
}

/// The current status of the autoroute operation.
#[derive(Clone, Copy, Debug)]
pub enum PlanarAutorouteContinueStatus {
    /// The autoroute is currently running and in progress.
    Running,
    /// A band has been successfully routed.
    Routed(BandTermsegIndex),
    /// A band had been already routed and has been skipped.
    Skipped(BandTermsegIndex),
}

/// Manages the autorouting process across multiple ratlines.
#[derive(Getters)]
pub struct PlanarAutorouteExecutionStepper {
    /// The stepper configuration, including the ratlines which we are routing.
    configuration: PlanarAutorouteConfiguration,
    /// Keeps track of the current ratline being routed, if one is active.
    curr_ratline_index: usize,
    /// Stores the current route being processed, if any.
    route: Option<RouteStepper>,
    /// Records the changes to the layout, one routed band per item.
    layout_edits: Vec<LayoutEdit>,
    /// Records the changes to the board data, one routed band per item.
    board_data_edits: Vec<BoardDataEdit>,
    /// The options for the autorouting process, defining how routing should be carried out.
    options: PlanarAutorouteOptions,
}

impl PlanarAutorouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        configuration: PlanarAutorouteConfiguration,
        options: PlanarAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        if configuration.ratlines.is_empty() {
            return Err(AutorouterError::NothingToRoute);
        };

        let (origin, destination) = configuration.ratline_terminating_dots(autorouter, 0);
        let mut router = Router::new(autorouter.board.layout_mut(), options.router);

        Ok(Self {
            configuration,
            curr_ratline_index: 0,
            route: Some(router.route(
                LayoutEdit::new(),
                origin,
                destination,
                options.router.routed_band_width,
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
            return Err(AutorouterError::NothingToUndoForReconfiguration);
        }

        self.dissolve_route_stepper_and_push_layout_edit();

        let board_edit = BoardEdit::new_from_edits(
            BoardDataEdit::merge_iter(self.board_data_edits.split_off(index)),
            LayoutEdit::merge_iter(self.layout_edits.split_off(index)),
        );

        autorouter.board.apply_edit(&board_edit.reverse());

        let (origin, destination) = self
            .configuration
            .ratline_terminating_dots(autorouter, index);
        let mut router = Router::new(autorouter.board.layout_mut(), self.options.router);

        self.route = Some(router.route(
            LayoutEdit::new(),
            origin,
            destination,
            self.options.router.routed_band_width,
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

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, PlanarAutorouteContinueStatus>
    for PlanarAutorouteExecutionStepper
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, PlanarAutorouteContinueStatus>, AutorouterError>
    {
        // TODO: Use a proper state machine here for better readability?

        if self.curr_ratline_index >= self.configuration().ratlines.len() {
            self.dissolve_route_stepper_and_push_layout_edit();

            return Ok(ControlFlow::Break(Some(BoardEdit::new_from_edits(
                BoardDataEdit::merge_iter(self.board_data_edits.iter().cloned()),
                // FIXME: This is a large clone. We probably need some
                // high-level, say `AutorouteEdit`, struct to store sequences
                // of edits.
                LayoutEdit::merge_iter(self.layout_edits.iter().cloned()),
            ))));
        }

        let (origin, destination) = self
            .configuration
            .ratline_terminating_dots(autorouter, self.curr_ratline_index);

        let Some(ref mut route) = self.route else {
            // May happen if stepper was aborted.
            return Ok(ControlFlow::Break(None));
        };

        let ret = if let Some(band_termseg) =
            autorouter.board.band_between_nodes(origin, destination)
        {
            PlanarAutorouteContinueStatus::Skipped(band_termseg[false])
        } else {
            let band_termseg = {
                let mut router = Router::new(autorouter.board.layout_mut(), self.options.router);

                let ControlFlow::Break(band_termseg) = route.step(&mut router)? else {
                    return Ok(ControlFlow::Continue(
                        PlanarAutorouteContinueStatus::Running,
                    ));
                };
                band_termseg
            };

            let band = autorouter
                .board
                .layout()
                .drawing()
                .find_loose_band_uid(band_termseg.into())
                .expect("a completely routed band should've Seg's as ends");

            autorouter
                .ratsnests
                .on_principal_layer_mut(self.options.principal_layer)
                .assign_band_termseg_to_ratline(
                    self.configuration().ratlines[self.curr_ratline_index].index,
                    band_termseg,
                );

            let mut board_data_edit = BoardDataEdit::new();

            autorouter.board.try_set_band_between_nodes(
                &mut board_data_edit,
                origin,
                destination,
                band,
            );

            self.board_data_edits.push(board_data_edit);

            PlanarAutorouteContinueStatus::Routed(band_termseg)
        };

        self.curr_ratline_index += 1;

        if let Some(new_ratline) = self.configuration.ratlines.get(self.curr_ratline_index) {
            let (origin, destination) = self
                .configuration
                .ratline_terminating_dots(autorouter, self.curr_ratline_index);
            let mut router = Router::new(autorouter.board.layout_mut(), self.options.router);

            self.dissolve_route_stepper_and_push_layout_edit();
            let recorder = LayoutEdit::new();

            self.route = Some(router.route(
                recorder,
                origin,
                destination,
                self.options.router.routed_band_width,
            )?);
        }

        Ok(ControlFlow::Continue(ret))
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for PlanarAutorouteExecutionStepper {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        self.backtrace_to_index(autorouter, 0);
        self.curr_ratline_index = self.configuration.ratlines.len();
    }
}

impl<M: AccessMesadata> Reconfigure<Autorouter<M>> for PlanarAutorouteExecutionStepper {
    type Configuration = PlanarAutorouteConfiguration;
    type Output = Result<PlanarAutorouteConfigurationStatus, AutorouterError>;

    fn reconfigure(
        &mut self,
        autorouter: &mut Autorouter<M>,
        new_configuration: PlanarAutorouteConfiguration,
    ) -> Result<PlanarAutorouteConfigurationStatus, AutorouterError> {
        let Some(new_index) = new_configuration
            .ratlines
            .iter()
            .zip(self.configuration.ratlines.iter())
            .position(|(permuted, original)| *permuted != *original)
        else {
            return Err(AutorouterError::NothingToUndoForReconfiguration);
        };

        let result = PlanarAutorouteConfigurationStatus {
            configuration: std::mem::replace(&mut self.configuration, new_configuration),
            costs: PlanarAutorouteCosts {
                lengths: vec![], // TODO.
            },
        };

        self.backtrace_to_index(autorouter, new_index)?;
        Ok(result)
    }
}

impl EstimateProgress for PlanarAutorouteExecutionStepper {
    type Value = f64;

    fn estimate_progress_value(&self) -> f64 {
        self.curr_ratline_index as f64
            + self.route.as_ref().map_or(0.0, |route| {
                route.estimate_progress_value() / route.estimate_progress_maximum()
            })
    }

    fn estimate_progress_maximum(&self) -> f64 {
        self.configuration().ratlines.len() as f64
    }
}

impl GetDebugOverlayData for PlanarAutorouteExecutionStepper {
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
