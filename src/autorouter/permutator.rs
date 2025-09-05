// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, iter::Skip, ops::ControlFlow};

use itertools::{Itertools, Permutations};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        autoroute::{AutorouteContinueStatus, AutorouteExecutionStepper},
        invoker::GetDebugOverlayData,
        ratline::RatlineIndex,
        Autorouter, AutorouterError, AutorouterOptions, PresortBy,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, Permutate, Step},
};

pub struct AutorouteExecutionPermutator {
    stepper: AutorouteExecutionStepper,
    permutations_iter: Skip<Permutations<std::vec::IntoIter<RatlineIndex>>>,
    options: AutorouterOptions,
}

impl AutorouteExecutionPermutator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        mut ratlines: Vec<RatlineIndex>,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let ratlines_len = ratlines.len();

        match options.presort_by {
            PresortBy::RatlineIntersectionCountAndLength => ratlines.sort_unstable_by(|a, b| {
                let a_intersector_count = a.ref_(autorouter).interior_obstacle_ratlines().count();
                let b_intersector_count = b.ref_(autorouter).interior_obstacle_ratlines().count();

                let primary_ordering = a_intersector_count.cmp(&b_intersector_count);

                if primary_ordering != Ordering::Equal {
                    primary_ordering
                } else {
                    let a_length = a.ref_(autorouter).length();
                    let b_length = b.ref_(autorouter).length();
                    let secondary_ordering = a_length.total_cmp(&b_length);

                    secondary_ordering
                }
            }),
            PresortBy::PairwiseDetours => ratlines.sort_unstable_by(|a, b| {
                let mut compare_detours = autorouter
                    .compare_detours_ratlines(*a, *b, options)
                    .unwrap();

                if let Ok((al, bl)) = compare_detours.finish(autorouter) {
                    PartialOrd::partial_cmp(&al, &bl).unwrap()
                } else {
                    Ordering::Equal
                }
            }),
        }

        Ok(Self {
            stepper: AutorouteExecutionStepper::new(autorouter, ratlines.clone(), options)?,
            // Note: I assume here that the first permutation is the same as the original order.
            permutations_iter: ratlines.into_iter().permutations(ratlines_len).skip(1),
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
                    let Some(permutation) = self.permutations_iter.next() else {
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
        self.permutations_iter.all(|_| true);
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
