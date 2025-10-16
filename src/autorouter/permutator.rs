// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        invoker::GetDebugOverlayData,
        permuter::{PermuteRatlines, RatlinesPermuter},
        planar_autoroute::{PlanarAutorouteContinueStatus, PlanarAutorouteExecutionStepper},
        presorter::{PresortParams, PresortRatlines, SccIntersectionsAndLengthPresorter},
        ratline::RatlineUid,
        Autorouter, AutorouterError, PlanarAutorouteOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, Permutate, Step},
};

pub struct PlanarAutorouteExecutionPermutator {
    stepper: PlanarAutorouteExecutionStepper,
    permuter: RatlinesPermuter,
    options: PlanarAutorouteOptions,
}

impl PlanarAutorouteExecutionPermutator {
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
        let permuter = RatlinesPermuter::new(autorouter, ratlines, presorter, &options);

        Ok(Self {
            stepper: PlanarAutorouteExecutionStepper::new(
                autorouter,
                initially_sorted_ratlines,
                options,
            )?,
            // Note: I assume here that the first permutation is the same as the original order.
            permuter,
            options,
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, PlanarAutorouteContinueStatus>
    for PlanarAutorouteExecutionPermutator
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, PlanarAutorouteContinueStatus>, AutorouterError>
    {
        match self.stepper.step(autorouter) {
            Ok(ok) => Ok(ok),
            Err(err) => {
                if !self.options.permutate {
                    return Err(err);
                }

                loop {
                    let Some(permutation) =
                        self.permuter.permute_ratlines(autorouter, &self.stepper)
                    else {
                        return Ok(ControlFlow::Break(None));
                    };

                    match self.stepper.permutate(autorouter, permutation) {
                        Ok(()) => break,
                        Err(AutorouterError::NothingToUndoForPermutation) => continue,
                        Err(err) => return Err(err),
                    }
                }

                self.stepper.step(autorouter)
            }
        }
    }
}

impl<M: AccessMesadata> Abort<Autorouter<M>> for PlanarAutorouteExecutionPermutator {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        //self.permutations_iter.all(|_| true); // Why did I add this code here???
        self.stepper.abort(autorouter);
    }
}

impl EstimateProgress for PlanarAutorouteExecutionPermutator {
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

impl GetDebugOverlayData for PlanarAutorouteExecutionPermutator {
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
