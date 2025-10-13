// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::iter::Skip;

use enum_dispatch::enum_dispatch;
use itertools::{Itertools, Permutations};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        planar_autoroute::PlanarAutorouteExecutionStepper,
        presorter::SccIntersectionsAndLengthPresorter, ratline::RatlineIndex, scc::Scc, Autorouter,
        PlanarAutorouteOptions,
    },
    drawing::graph::MakePrimitiveRef,
    geometry::{GenericNode, GetLayer},
    graph::MakeRef,
};

#[enum_dispatch]
pub trait PermuteRatlines {
    fn permute_ratlines(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<Vec<RatlineIndex>>;
}

#[enum_dispatch(PermuteRatlines)]
pub enum RatlinesPermuter {
    RatlineCuts(RatlineCutsRatlinePermuter),
    SccPermutations(SccPermutationsRatlinePermuter),
}

impl RatlinesPermuter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        presorter: SccIntersectionsAndLengthPresorter,
        options: &PlanarAutorouteOptions,
    ) -> Self {
        RatlinesPermuter::SccPermutations(SccPermutationsRatlinePermuter::new(
            autorouter, ratlines, presorter, options,
        ))
        /*RatlinesPermuter::RatlineCuts(RatlineCutsRatlinePermuter::new(
            autorouter, ratlines, presorter, options,
        ))*/
    }
}

pub struct SccPermutationsRatlinePermuter {
    sccs_permutations_iter: Skip<Permutations<std::vec::IntoIter<Scc>>>,
    original_ratlines: Vec<RatlineIndex>,
}

impl SccPermutationsRatlinePermuter {
    pub fn new(
        _autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: Vec<RatlineIndex>,
        presorter: SccIntersectionsAndLengthPresorter,
        _options: &PlanarAutorouteOptions,
    ) -> Self {
        // TODO: Instead of instantiating presorter again here, get it from
        // an argument.
        let sccs = presorter.dissolve();
        let sccs_len = sccs.len();

        Self {
            sccs_permutations_iter: sccs.into_iter().permutations(sccs_len).skip(1),
            original_ratlines: ratlines,
        }
    }
}

impl PermuteRatlines for SccPermutationsRatlinePermuter {
    fn permute_ratlines(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        _stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<Vec<RatlineIndex>> {
        let scc_permutation = self.sccs_permutations_iter.next()?;
        let mut ratlines = vec![];

        for scc in scc_permutation {
            for ratline in self.original_ratlines.iter() {
                if scc.node_indices().contains(
                    &autorouter
                        .ratsnests()
                        .on_principal_layer(0)
                        .graph()
                        .edge_endpoints(*ratline)
                        .unwrap()
                        .0,
                ) && scc.node_indices().contains(
                    &autorouter
                        .ratsnests()
                        .on_principal_layer(0)
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

pub struct RatlineCutsRatlinePermuter {
    //sccs: Vec<Vec<NodeIndex<usize>>>,
}

impl RatlineCutsRatlinePermuter {
    pub fn new(
        _autorouter: &mut Autorouter<impl AccessMesadata>,
        _ratlines: Vec<RatlineIndex>,
        _presorter: SccIntersectionsAndLengthPresorter,
        _options: &PlanarAutorouteOptions,
    ) -> Self {
        /*Self {
            sccs: presorter.dissolve(),
        }*/
        Self {}
    }
}

impl PermuteRatlines for RatlineCutsRatlinePermuter {
    fn permute_ratlines(
        &mut self,
        autorouter: &mut Autorouter<impl AccessMesadata>,
        stepper: &PlanarAutorouteExecutionStepper,
    ) -> Option<Vec<RatlineIndex>> {
        let curr_ratline = stepper.ratlines()[*stepper.curr_ratline_index()];
        let terminating_dots = curr_ratline.ref_(autorouter).terminating_dots();
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
            .ratlines()
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
        let mut ratlines = stepper.ratlines().clone();
        ratlines.swap(*stepper.curr_ratline_index(), first_cut_ratline_index);

        Some(ratlines)
    }
}
