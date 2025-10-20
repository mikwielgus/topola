// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        invoker::GetDebugOverlayData,
        planar_autoroute::{
            PlanarAutorouteConfigurationResult, PlanarAutorouteContinueStatus,
            PlanarAutorouteExecutionStepper,
        },
        planar_reconfigurer::{PermuteRatlines, PlanarReconfigurer},
        presorter::{PresortParams, PresortRatlines, SccIntersectionsAndLengthPresorter},
        ratline::RatlineUid,
        Autorouter, AutorouterError, PlanarAutorouteOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, ReconfiguratorStatus, Reconfigure, Step},
};
pub type PlanarReconfiguratorStatus =
    ReconfiguratorStatus<PlanarAutorouteConfigurationResult, PlanarAutorouteContinueStatus>;

pub struct PlanarAutorouteReconfigurator {
    stepper: PlanarAutorouteExecutionStepper,
    reconfigurer: PlanarReconfigurer,
    options: PlanarAutorouteOptions,
}

impl PlanarAutorouteReconfigurator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineUid>,
        options: PlanarAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let presorter = SccIntersectionsAndLengthPresorter::new(
            autorouter,
            &ratlines,
            &PresortParams {
                intersector_count_weight: 1.0,
                length_weight: 0.001,
            },
            &options,
        );
        let initially_sorted_ratlines = presorter.presort_ratlines(autorouter, &ratlines);
        /*let permuter = RatlinesPermuter::SccPermutations(SccPermutationsRatlinePermuter::new(
            autorouter, ratlines, presorter, &options,
        ));*/
        let reconfigurer = PlanarReconfigurer::new(autorouter, ratlines, presorter, &options);

        Ok(Self {
            stepper: PlanarAutorouteExecutionStepper::new(
                autorouter,
                initially_sorted_ratlines,
                options,
            )?,
            // Note: I assume here that the first permutation is the same as the original order.
            reconfigurer,
            options,
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, PlanarReconfiguratorStatus>
    for PlanarAutorouteReconfigurator
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, PlanarReconfiguratorStatus>, AutorouterError> {
        match self.stepper.step(autorouter) {
            Ok(ControlFlow::Break(maybe_edit)) => Ok(ControlFlow::Break(maybe_edit)),
            Ok(ControlFlow::Continue(status)) => {
                Ok(ControlFlow::Continue(ReconfiguratorStatus::Running(status)))
            }
            Err(err) => {
                if !self.options.permutate {
                    return Err(err);
                }

                loop {
                    let Some(permutation) = self
                        .reconfigurer
                        .permute_ratlines(autorouter, &self.stepper)
                    else {
                        return Ok(ControlFlow::Break(None));
                    };

                    match self.stepper.reconfigure(autorouter, permutation) {
                        Ok(result) => {
                            return Ok(ControlFlow::Continue(ReconfiguratorStatus::Reconfigured(
                                result,
                            )))
                        }
                        Err(AutorouterError::NothingToUndoForReconfiguration) => continue,
                        Err(err) => return Err(err),
                    }
                }
            }
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for PlanarAutorouteReconfigurator {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        //self.permutations_iter.all(|_| true); // Why did I add this code here???
        self.stepper.abort(autorouter);
    }
}

impl EstimateProgress for PlanarAutorouteReconfigurator {
    type Value = f64;

    fn estimate_progress_value(&self) -> f64 {
        // TODO.
        self.stepper.estimate_progress_value()
    }

    fn estimate_progress_maximum(&self) -> f64 {
        // TODO.
        self.stepper.estimate_progress_maximum()
    }
}

impl GetDebugOverlayData for PlanarAutorouteReconfigurator {
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        self.stepper.maybe_thetastar()
    }

    fn maybe_navcord(&self) -> Option<&Navcord> {
        self.stepper.maybe_navcord()
    }

    fn ghosts(&self) -> &[PrimitiveShape] {
        self.stepper.ghosts()
    }

    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.stepper.obstacles()
    }
}
