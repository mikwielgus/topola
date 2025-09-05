// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, ops::ControlFlow};

use itertools::{Itertools, Permutations};
use petgraph::{algo::tarjan_scc, graph::NodeIndex};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        autoroute::{AutorouteContinueStatus, AutorouteExecutionStepper},
        invoker::GetDebugOverlayData,
        ratline::RatlineIndex,
        Autorouter, AutorouterError, AutorouterOptions,
    },
    board::edit::BoardEdit,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navcord::Navcord, navmesh::Navmesh, thetastar::ThetastarStepper},
    stepper::{Abort, EstimateProgress, Permutate, Step},
};

struct RatlineSccPermuter {
    sccs_permutations_iter: Permutations<std::vec::IntoIter<Vec<NodeIndex<usize>>>>,
    ratlines: Vec<RatlineIndex>,
}

impl RatlineSccPermuter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        _options: &AutorouterOptions,
    ) -> Self {
        // FIXME: Unnecessary copy.
        let mut filtered_ratsnest = autorouter.ratsnest().graph().clone();
        filtered_ratsnest.retain_edges(|_g, i| ratlines.contains(&i));

        let mut sccs = tarjan_scc(&filtered_ratsnest);
        let sccs_len = sccs.len();

        sccs.sort_unstable_by(|a, b| {
            // FIXME: These calculations should probably be stored somewhere
            // instead of being done every time.

            let mut a_intersector_count = 0;
            let mut b_intersector_count = 0;
            let mut a_length = 0.0;
            let mut b_length = 0.0;

            // FIXME: It's inefficient to iterate over the ratlines on every
            // sort comparison. But this is the simplest solution I arrived
            // at after realizing that `.tarjan_scc(...)` does not sort nodes
            // inside components.
            for ratline in ratlines.iter() {
                if a.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().0)
                    && a.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().1)
                {
                    a_length += ratline.ref_(autorouter).length();
                    a_intersector_count += ratline
                        .ref_(autorouter)
                        .interior_obstacle_ratlines()
                        .count();
                }
            }

            for ratline in ratlines.iter() {
                if b.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().0)
                    && b.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().1)
                {
                    b_length += ratline.ref_(autorouter).length();
                    b_intersector_count += ratline
                        .ref_(autorouter)
                        .interior_obstacle_ratlines()
                        .count();
                }
            }

            let primary_ordering = a_intersector_count.cmp(&b_intersector_count);

            if primary_ordering != Ordering::Equal {
                primary_ordering
            } else {
                let secondary_ordering = a_length.total_cmp(&b_length);

                secondary_ordering
            }

            // Below is how I tried to do this before I realized that
            // `.tarjan_scc(...)` does not sort nodes inside components.

            /*let a_intersector_count: usize = a
                .windows(2)
                .map(|window| {
                    let ratline = filtered_ratsnest.find_edge(window[0], window[1]).unwrap();
                    ratline
                        .ref_(autorouter)
                        .interior_obstacle_ratlines()
                        .count()
                })
                .sum();
            let b_intersector_count: usize = b
                .windows(2)
                .map(|window| {
                    let ratline = filtered_ratsnest.find_edge(window[0], window[1]).unwrap();
                    ratline
                        .ref_(autorouter)
                        .interior_obstacle_ratlines()
                        .count()
                })
                .sum();

            let primary_ordering = a_intersector_count.cmp(&b_intersector_count);

            if primary_ordering != Ordering::Equal {
                primary_ordering
            } else {
                let a_length: f64 = a
                    .windows(2)
                    .map(|window| {
                        let ratline = filtered_ratsnest.find_edge(window[0], window[1]).unwrap();
                        ratline.ref_(autorouter).length()
                    })
                    .sum();
                let b_length: f64 = b
                    .windows(2)
                    .map(|window| {
                        let ratline = filtered_ratsnest.find_edge(window[0], window[1]).unwrap();
                        ratline.ref_(autorouter).length()
                    })
                    .sum();

                let secondary_ordering = a_length.total_cmp(&b_length);

                secondary_ordering
            }*/
        });

        Self {
            sccs_permutations_iter: sccs.into_iter().permutations(sccs_len),
            ratlines,
        }
    }

    pub fn next_permutation(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Option<Vec<RatlineIndex>> {
        let scc_permutation = self.sccs_permutations_iter.next()?;
        let mut ratlines = vec![];

        for scc in scc_permutation {
            for ratline in self.ratlines.iter() {
                if scc.contains(
                    &autorouter
                        .ratsnest()
                        .graph()
                        .edge_endpoints(*ratline)
                        .unwrap()
                        .0,
                ) && scc.contains(
                    &autorouter
                        .ratsnest()
                        .graph()
                        .edge_endpoints(*ratline)
                        .unwrap()
                        .1,
                ) {
                    ratlines.push(*ratline);
                }
            }
        }

        Some(ratlines)
    }
}

pub struct AutorouteExecutionPermutator {
    stepper: AutorouteExecutionStepper,
    permuter: RatlineSccPermuter,
    options: AutorouterOptions,
}

impl AutorouteExecutionPermutator {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let mut permuter = RatlineSccPermuter::new(autorouter, ratlines, &options);
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
