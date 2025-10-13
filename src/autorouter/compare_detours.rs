// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages the comparison of detours between two ratlines, tracking their
//! routing statuses and recording their lengths.

use std::ops::ControlFlow;

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{EstimateProgress, Step},
};

use super::{
    invoker::GetDebugOverlayData,
    planar_autoroute::{PlanarAutorouteContinueStatus, PlanarAutorouteExecutionStepper},
    ratline::RatlineIndex,
    Autorouter, AutorouterError, PlanarAutorouteOptions,
};

pub struct CompareDetoursExecutionStepper {
    autoroute: PlanarAutorouteExecutionStepper,
    next_autoroute: Option<PlanarAutorouteExecutionStepper>,
    ratline1: RatlineIndex,
    ratline2: RatlineIndex,
    total_length1: f64,
    total_length2: f64,
    done: bool,
}

impl CompareDetoursExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratline1: RatlineIndex,
        ratline2: RatlineIndex,
        options: PlanarAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        Ok(Self {
            autoroute: autorouter.planar_autoroute_ratlines(vec![ratline1, ratline2], options)?,
            next_autoroute: Some(
                autorouter.planar_autoroute_ratlines(vec![ratline2, ratline1], options)?,
            ),
            ratline1,
            ratline2,
            total_length1: 0.0,
            total_length2: 0.0,
            done: false,
        })
    }
}

// XXX: Do we really need this to be a stepper? We don't use at the moment, as sorting functions
// aren't steppable either. It may be useful for debugging later on tho.
impl<M: AccessMesadata> Step<Autorouter<M>, (f64, f64)> for CompareDetoursExecutionStepper {
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<(f64, f64)>, AutorouterError> {
        if self.done {
            return Ok(ControlFlow::Break((self.total_length1, self.total_length2)));
        }

        match self.autoroute.step(autorouter)? {
            ControlFlow::Continue(
                PlanarAutorouteContinueStatus::Running | PlanarAutorouteContinueStatus::Skipped(_),
            ) => Ok(ControlFlow::Continue(())),
            ControlFlow::Continue(PlanarAutorouteContinueStatus::Routed(band_termseg)) => {
                let length = band_termseg
                    .ref_(autorouter.board.layout().drawing())
                    .length();

                if self.next_autoroute.is_some() {
                    self.total_length1 += length;
                } else {
                    self.total_length2 += length;
                }

                Ok(ControlFlow::Continue(()))
            }
            ControlFlow::Break(..) => {
                if let Some(next_autoroute) = self.next_autoroute.take() {
                    autorouter
                        .undo_planar_autoroute_ratlines(vec![self.ratline1, self.ratline2])?;
                    self.autoroute = next_autoroute;

                    Ok(ControlFlow::Continue(()))
                } else {
                    self.done = true;
                    autorouter
                        .undo_planar_autoroute_ratlines(vec![self.ratline2, self.ratline1])?;

                    Ok(ControlFlow::Break((self.total_length1, self.total_length2)))
                }
            }
        }
    }
}

impl EstimateProgress for CompareDetoursExecutionStepper {
    type Value = f64;
}

impl GetDebugOverlayData for CompareDetoursExecutionStepper {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.autoroute.maybe_thetastar()
    }

    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.autoroute.maybe_navcord()
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        self.autoroute.ghosts()
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.autoroute.obstacles()
    }
}
