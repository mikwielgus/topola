// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use petgraph::algo::tarjan_scc;
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{ratline::RatlineIndex, scc::Scc, Autorouter};

pub struct PresortParams {
    pub intersector_count_weight: f64,
    pub length_weight: f64,
}

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
        params: &PresortParams,
    ) -> Self {
        // FIXME: Unnecessary copy.
        let mut filtered_ratsnest = autorouter.ratsnests().on_principal_layer(0).graph().clone();
        filtered_ratsnest.retain_edges(|_g, i| ratlines.contains(&i));

        let mut sccs: Vec<_> = tarjan_scc(&filtered_ratsnest)
            .into_iter()
            .map(|node_indices| Scc::new(autorouter, ratlines, &filtered_ratsnest, node_indices))
            .collect();

        sccs.sort_unstable_by(|a, b| {
            Self::scc_score(params, a).total_cmp(&Self::scc_score(params, b))
        });

        Self { sccs }
    }

    fn scc_score(params: &PresortParams, scc: &Scc) -> f64 {
        params.intersector_count_weight * *scc.intersector_count() as f64
            + params.length_weight * scc.length()
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
