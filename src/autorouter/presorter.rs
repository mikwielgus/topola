// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::cmp::Ordering;

use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use petgraph::{algo::tarjan_scc, graph::NodeIndex};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{ratline::RatlineIndex, Autorouter},
    geometry::shape::MeasureLength,
    graph::MakeRef,
};

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
    sccs: Vec<Vec<NodeIndex<usize>>>,
}

impl SccIntersectionsAndLengthPresorter {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: &[RatlineIndex],
    ) -> Self {
        // FIXME: Unnecessary copy.
        let mut filtered_ratsnest = autorouter.ratsnest().graph().clone();
        filtered_ratsnest.retain_edges(|_g, i| ratlines.contains(&i));

        let mut sccs = tarjan_scc(&filtered_ratsnest);

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
                    a_intersector_count +=
                        ratline.ref_(autorouter).interiorly_cut_ratlines().count();
                    a_intersector_count +=
                        ratline.ref_(autorouter).cut_other_net_primitives().count();
                }
            }

            for ratline in ratlines.iter() {
                if b.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().0)
                    && b.contains(&filtered_ratsnest.edge_endpoints(*ratline).unwrap().1)
                {
                    b_length += ratline.ref_(autorouter).length();
                    b_intersector_count +=
                        ratline.ref_(autorouter).interiorly_cut_ratlines().count();
                    b_intersector_count +=
                        ratline.ref_(autorouter).cut_other_net_primitives().count();
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
                    presorted_ratlines.push(*ratline);
                }
            }
        }

        presorted_ratlines
    }
}
