// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        autoroute::{AutorouteContinueStatus, AutorouteExecutionStepper},
        invoker::GetDebugOverlayData,
        permuter::{PermuteRatlines, RatlinePermuter, SccRatlinePermuter},
        ratline::RatlineIndex,
        Autorouter, AutorouterError, AutorouterOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, Permutate, Step},
};

pub struct AutorouteExecutionPermutator {
    stepper: AutorouteExecutionStepper,
    permuter: RatlinePermuter,
    options: AutorouterOptions,
}

impl AutorouteExecutionPermutator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let mut permuter =
            RatlinePermuter::Scc(SccRatlinePermuter::new(autorouter, ratlines, &options));
        let initially_sorted_ratlines = permuter.next_permutation(autorouter).unwrap();

        Ok(Self {
            stepper: AutorouteExecutionStepper::new(
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

impl<M: AccessMesadata> Step<Autorouter<M>, Option<BoardEdit>, AutorouteContinueStatus>
    for AutorouteExecutionPermutator
{
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<Option<BoardEdit>, AutorouteContinueStatus>, AutorouterError> {
        match self.stepper.step(autorouter) {
            Ok(ok) => Ok(ok),
            Err(err) => {
                if !self.options.permutate {
                    return Err(err);
                }

                loop {
                    let Some(permutation) = self.permuter.next_permutation(autorouter) else {
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

impl<M: AccessMesadata> Abort<Autorouter<M>> for AutorouteExecutionPermutator {
    fn abort(&mut self, autorouter: &mut Autorouter<M>) {
        //self.permutations_iter.all(|_| true); // Why did I add this code here???
        self.stepper.abort(autorouter);
    }
}

impl EstimateProgress for AutorouteExecutionPermutator {
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

impl GetDebugOverlayData for AutorouteExecutionPermutator {
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
