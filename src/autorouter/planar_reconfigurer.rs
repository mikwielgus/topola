// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    cmp::Ordering,
    iter::{Skip, Take},
};

use derive_getters::Getters;
use enum_dispatch::enum_dispatch;
use itertools::{Itertools, Permutations};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    astar::Astar,
    autorouter::{
        planar_autoroute::{PlanarAutorouteConfiguration, PlanarAutorouteExecutionStepper},
        planar_preconfigurer::SccIntersectionsAndLengthRatlinePlanarAutoroutePreconfigurer,
        scc::Scc,
        Autorouter, PlanarAutorouteOptions,
    },
};

#[enum_dispatch]
pub trait MakeNextPlanarAutorouteConfiguration {
    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
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

#[derive(Clone, Debug, Getters)]
struct SccSearchNode {
    curr_permutation: Vec<Scc>,
    #[getter(skip)]
    permutations: Skip<Permutations<Take<std::vec::IntoIter<Scc>>>>,
    #[getter(skip)]
    length: usize,
}

impl Ord for SccSearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.curr_permutation.cmp(&other.curr_permutation)
    }
}

impl PartialOrd for SccSearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for SccSearchNode {}

impl PartialEq for SccSearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.curr_permutation == other.curr_permutation
    }
}

impl SccSearchNode {
    pub fn new(sccs: Vec<Scc>) -> Self {
        let len = sccs.len();

        Self {
            curr_permutation: sccs.clone(),
            permutations: sccs.into_iter().take(len).permutations(0).skip(0),
            length: 0,
        }
    }

    pub fn expand(&self, length: usize) -> Vec<(f64, f64, Self)> {
        let mut expanded_nodes = vec![];

        if let Some(resized) = self.clone().resize(length) {
            if let Some(permuted_resized) = resized.permute() {
                expanded_nodes.push((
                    0.1,
                    (self.curr_permutation.len() - length) as f64,
                    permuted_resized,
                ));
            }
        }

        if let Some(permuted) = self.clone().permute() {
            expanded_nodes.push((
                0.1,
                (self.curr_permutation.len() - self.length) as f64,
                permuted,
            ));
        }

        expanded_nodes
    }

    fn resize(self, length: usize) -> Option<Self> {
        if length == self.length {
            return None;
        }

        Some(Self {
            curr_permutation: self.curr_permutation.clone(),
            permutations: self
                .curr_permutation
                .into_iter()
                .take(length)
                .permutations(length)
                .skip(1),
            length,
        })
    }

    fn permute(mut self) -> Option<Self> {
        for (i, element) in self.permutations.next()?.iter().enumerate() {
            self.curr_permutation[i] = element.clone();
        }

        Some(self)
    }
}

pub struct SccPermutationsPlanarAutorouteReconfigurer {
    sccs_search: Astar<SccSearchNode, f64>,
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

        Self {
            sccs_search: Astar::new(SccSearchNode::new(sccs)),
            preconfiguration,
        }
    }
}

impl MakeNextPlanarAutorouteConfiguration for SccPermutationsPlanarAutorouteReconfigurer {
    fn next_configuration(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<PlanarAutorouteConfiguration> {
        let scc_index = self
            .sccs_search
            .curr_node()
            .curr_permutation()
            .iter()
            .position(|scc| {
                scc.scc_ref(autorouter)
                    .contains(self.preconfiguration.ratlines[*stepper.curr_ratline_index()])
            })
            .unwrap();

        let next_search_node = self
            .sccs_search
            .expand(&self.sccs_search.curr_node().expand(scc_index + 1))?;
        let next_permutation = next_search_node.curr_permutation();
        let mut ratlines = vec![];

        for scc in next_permutation {
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
