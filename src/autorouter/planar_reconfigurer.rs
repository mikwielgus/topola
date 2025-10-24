// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::iter::Skip;

use enum_dispatch::enum_dispatch;
use itertools::{Itertools, Permutations};
use specctra_core::mesadata::AccessMesadata;

use crate::autorouter::{
    planar_autoroute::{PlanarAutorouteConfiguration, PlanarAutorouteExecutionStepper},
    planar_preconfigurer::SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer,
    ratline::RatlineUid,
    scc::Scc,
    Autorouter, PlanarAutorouteOptions,
};

#[enum_dispatch]
pub trait MakeNextPlanarAutorouteConfiguration {
    fn next_configuration(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<PlanarAutorouteConfiguration>;
}

#[enum_dispatch(MakeNextPlanarAutorouteConfiguration)]
pub enum PlanarAutorouteReconfigurer {
    //RatlineCuts(RatlineCutsPlanarAutorouteReconfigurer),
    SccPermutations(SccPermutationsPlanarAutorouteReconfigurer),
}

impl PlanarAutorouteReconfigurer {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        preconfiguration: PlanarAutorouteConfiguration,
        presorter: SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer,
        options: &PlanarAutorouteOptions,
    ) -> Self {
        PlanarAutorouteReconfigurer::SccPermutations(
            SccPermutationsPlanarAutorouteReconfigurer::new(
                autorouter,
                preconfiguration,
                presorter,
                options,
            ),
        )
        /*RatlinesPermuter::RatlineCuts(RatlineCutsRatlinePermuter::new(
            autorouter, ratlines, presorter, options,
        ))*/
    }
}

pub struct SccPermutationsPlanarAutorouteReconfigurer {
    sccs_permutations_iter: Skip<Permutations<std::vec::IntoIter<Scc>>>,
    preconfiguration: PlanarAutorouteConfiguration,
}

impl SccPermutationsPlanarAutorouteReconfigurer {
    pub fn new(
        _autorouter: &mut Autorouter<impl AccessMesadata>,
        preconfiguration: PlanarAutorouteConfiguration,
        presorter: SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer,
        _options: &PlanarAutorouteOptions,
    ) -> Self {
        // TODO: Instead of instantiating presorter again here, get it from
        // an argument.
        let sccs = presorter.dissolve();
        let sccs_len = sccs.len();

        Self {
            sccs_permutations_iter: sccs.into_iter().permutations(sccs_len).skip(1),
            preconfiguration,
        }
    }
}

impl MakeNextPlanarAutorouteConfiguration for SccPermutationsPlanarAutorouteReconfigurer {
    fn next_configuration(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        _stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<PlanarAutorouteConfiguration> {
        let scc_permutation = self.sccs_permutations_iter.next()?;
        let mut ratlines = vec![];

        for scc in scc_permutation {
            for ratline in self.preconfiguration.ratlines.iter() {
                if scc.node_indices().contains(
                    &autorouter
                        .ratsnests()
                        .on_principal_layer(ratline.principal_layer)
                        .graph()
                        .edge_endpoints(ratline.index)
                        .unwrap()
                        .0,
                ) && scc.node_indices().contains(
                    &autorouter
                        .ratsnests()
                        .on_principal_layer(ratline.principal_layer)
                        .graph()
                        .edge_endpoints(ratline.index)
                        .unwrap()
                        .1,
                ) {
                    ratlines.push(*ratline);
                }
            }
        }

        Some(PlanarAutorouteConfiguration {
            ratlines,
            ..self.preconfiguration.clone()
        })
    }
}

pub struct RatlineCutsPlanarAutorouteReconfigurer {
    //sccs: Vec<Vec<NodeIndex<usize>>>,
}

/*impl RatlineCutsPlanarAutorouteReconfigurer {
    pub fn new(
        _autorouter: &mut Autorouter<impl AccessMesadata>,
        _ratlines: Vec<RatlineUid>,
        _presorter: SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer,
        _options: &PlanarAutorouteOptions,
    ) -> Self {
        /*Self {
            sccs: presorter.dissolve(),
        }*/
        Self {}
    }
}*/

/*impl MakeNextPlanarAutorouteConfiguration for RatlineCutsPlanarAutorouteReconfigurer {
    fn next_configuration(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<Vec<RatlineUid>> {
        let curr_ratline = stepper.configuration().ratlines[*stepper.curr_ratline_index()];
        let terminating_dots = stepper
            .configuration()
            .terminating_dots
            .get(&(curr_ratline,));
        let bands_cut_by_ratline: Vec<_> = autorouter
            .board()
            .layout()
            .bands_between_nodes(
                terminating_dots
                    .0
                    .primitive_ref(autorouter.board().layout().drawing())
                    .layer(),
                GenericNode::Primitive(terminating_dots.0.into()),
                GenericNode::Primitive(terminating_dots.1.into()),
            )
            .collect();

        // Find the first ratline corresponding to a band that is cut.
        let first_cut_ratline_index = stepper
            .configuration()
            .ratlines
            .iter()
            .position(|ratline| {
                for (band_uid, _) in &bands_cut_by_ratline {
                    if ratline.ref_(autorouter).band_termseg() == band_uid.0
                        || ratline.ref_(autorouter).band_termseg() == band_uid.1
                    {
                        return true;
                    }
                }

                false
            })
            .unwrap();

        // Swap the first ratline corresponding to a band that is cut with the
        // ratline that we have failed routing.
        let mut ratlines = stepper.configuration().ratlines.clone();
        ratlines.swap(*stepper.curr_ratline_index(), first_cut_ratline_index);

        Some(ratlines)
    }
}*/
