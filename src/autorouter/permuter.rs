// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;
use itertools::{Itertools, Permutations};
use petgraph::graph::NodeIndex;
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    presorter::SccIntersectionsAndLengthPresorter, ratline::RatlineIndex, Autorouter,
    AutorouterOptions,
};

#[enum_dispatch]
pub trait PermuteRatlines {
    fn next_ratlines_permutation(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
    ) -> Option<Vec<RatlineIndex>>;
}

#[enum_dispatch(PermuteRatlines)]
pub enum RatlinesPermuter {
    SccPermutations(SccPermutationsRatlinePermuter),
}

impl RatlinesPermuter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        options: &AutorouterOptions,
    ) -> Self {
        RatlinesPermuter::SccPermutations(SccPermutationsRatlinePermuter::new(
            autorouter, ratlines, options,
        ))
    }
}

pub struct SccPermutationsRatlinePermuter {
    sccs_permutations_iter: Permutations<std::vec::IntoIter<Vec<NodeIndex<usize>>>>,
    ratlines: Vec<RatlineIndex>,
}

impl SccPermutationsRatlinePermuter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        _options: &AutorouterOptions,
    ) -> Self {
        // TODO: Instead of instantiating presorter again here, get it from
        // an argument.
        let presorter = SccIntersectionsAndLengthPresorter::new(autorouter, &ratlines);
        let sccs = presorter.dissolve();
        let sccs_len = sccs.len();

        Self {
            sccs_permutations_iter: sccs.into_iter().permutations(sccs_len),
            ratlines,
        }
    }
}

impl PermuteRatlines for SccPermutationsRatlinePermuter {
    fn next_ratlines_permutation(
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
