// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::cmp::Ordering;

use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use petgraph::algo::tarjan_scc;
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{ratline::RatlineIndex, scc::Scc, Autorouter};

#[enum_dispatch]
pub trait PresortRatlines {
    fn presort_ratlines(
        &self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: &[RatlineIndex],
    ) -> Vec<RatlineIndex>;
}

#[enum_dispatch(PresortRatlines)]
pub enum RatlinesPresorter {
    SccIntersectionsLength(SccIntersectionsAndLengthPresorter),
}

#[derive(Getters, Dissolve)]
pub struct SccIntersectionsAndLengthPresorter {
    sccs: Vec<Scc>,
}

impl SccIntersectionsAndLengthPresorter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: &[RatlineIndex],
    ) -> Self {
        // FIXME: Unnecessary copy.
        let mut filtered_ratsnest = autorouter.ratsnest().graph().clone();
        filtered_ratsnest.retain_edges(|_g, i| ratlines.contains(&i));

        let mut sccs: Vec<_> = tarjan_scc(&filtered_ratsnest)
            .into_iter()
            .map(|node_indices| Scc::new(autorouter, ratlines, &filtered_ratsnest, node_indices))
            .collect();

        sccs.sort_unstable_by(|a, b| {
            let primary_ordering = a.intersector_count().cmp(&b.intersector_count());

            if primary_ordering != Ordering::Equal {
                primary_ordering
            } else {
                let secondary_ordering = a.length().total_cmp(&b.length());

                secondary_ordering
            }
        });

        Self { sccs }
    }
}

impl PresortRatlines for SccIntersectionsAndLengthPresorter {
    fn presort_ratlines(
        &self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: &[RatlineIndex],
    ) -> Vec<RatlineIndex> {
        let mut presorted_ratlines = vec![];

        for scc in self.sccs.iter() {
            for ratline in ratlines.iter() {
                if scc.scc_ref(autorouter).contains(*ratline) {
                    presorted_ratlines.push(*ratline);
                }
            }
        }

        presorted_ratlines
    }
}
