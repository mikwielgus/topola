// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use petgraph::algo::tarjan_scc;
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    planar_autoroute::PlanarAutorouteConfiguration, ratline::RatlineUid, scc::Scc, Autorouter,
    PlanarAutorouteOptions,
};

#[derive(Clone, Debug)]
pub struct PlanarAutoroutePreconfigurerInput {
    pub ratlines: Vec<RatlineUid>,
}

pub struct PresortParams {
    pub intersector_count_weight: f64,
    pub length_weight: f64,
}

#[enum_dispatch]
pub trait PreconfigurePlanarAutoroute {
    fn preconfigure(
        &self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        input: PlanarAutoroutePreconfigurerInput,
    ) -> PlanarAutorouteConfiguration;
}

#[enum_dispatch(PresortRatlines)]
pub enum PlanarAutoroutePreconfigurer {
    SccIntersectionsLength(SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer),
}

#[derive(Getters, Dissolve)]
pub struct SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer {
    sccs: Vec<Scc>,
}

impl SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        input: PlanarAutoroutePreconfigurerInput,
        params: &PresortParams,
        options: &PlanarAutorouteOptions,
    ) -> Self {
        // FIXME: Unnecessary copy.
        let mut filtered_ratsnest = autorouter
            .ratsnests()
            .on_principal_layer(options.principal_layer)
            .graph()
            .clone();
        filtered_ratsnest
            .retain_edges(|_g, i| input.ratlines.iter().any(|ratline| ratline.index == i));

        let mut sccs: Vec<_> = tarjan_scc(&filtered_ratsnest)
            .into_iter()
            .map(|node_indices| {
                Scc::new(
                    autorouter,
                    &input.ratlines,
                    &filtered_ratsnest,
                    node_indices,
                )
            })
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

impl PreconfigurePlanarAutoroute for SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer {
    fn preconfigure(
        &self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        input: PlanarAutoroutePreconfigurerInput,
    ) -> PlanarAutorouteConfiguration {
        let mut presorted_ratlines = vec![];

        for scc in self.sccs.iter() {
            for ratline in input.ratlines.iter() {
                if scc.scc_ref(autorouter).contains(*ratline) {
                    presorted_ratlines.push(*ratline);
                }
            }
        }

        PlanarAutorouteConfiguration {
            ratlines: presorted_ratlines,
        }
    }
}
