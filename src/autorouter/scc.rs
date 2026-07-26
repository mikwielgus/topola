// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{cmp::Ordering, collections::BTreeSet};

use derive_getters::Getters;
use petgraph::{graph::NodeIndex, prelude::StableUnGraph};
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        ratline::{RatlineUid, RatlineWeight},
        ratsnest::RatvertexWeight,
        Autorouter,
    },
    geometry::shape::MeasureLength,
    graph::MakeRef,
};

#[derive(Clone, Debug, Getters)]
pub struct Scc {
    node_indices: Vec<NodeIndex<usize>>,

    intersector_count: usize,
    length: f64,
}

impl Ord for Scc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.node_indices.cmp(&other.node_indices)
    }
}

impl PartialOrd for Scc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Scc {
    fn eq(&self, other: &Self) -> bool {
        self.node_indices == other.node_indices
    }
}

impl Eq for Scc {}

impl Scc {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: &BTreeSet<RatlineUid>,
        filtered_ratsnest: &StableUnGraph<RatvertexWeight, RatlineWeight, usize>,
        node_indices: Vec<NodeIndex<usize>>,
    ) -> Self {
        let mut this = Self {
            node_indices,
            length: 0.0,
            intersector_count: 0,
        };

        for ratline in ratlines.iter() {
            let Some((a, b)) = filtered_ratsnest.edge_endpoints(ratline.index) else {
                // Ratline index may belong to another principal-layer ratsnest.
                continue;
            };
            if this.node_indices.contains(&a) && this.node_indices.contains(&b) {
                this.length += ratline.ref_(autorouter).length();
                this.intersector_count +=
                    ratline.ref_(autorouter).interiorly_cut_ratlines().count();
                this.intersector_count +=
                    ratline.ref_(autorouter).cut_other_net_primitives().count();
            }
        }

        this
    }

    pub fn scc_ref<'a, M: AccessMesadata>(
        &'a self,
        autorouter: &'a Autorouter<M>,
    ) -> SccRef<'a, M> {
        SccRef::new(self, autorouter)
    }
}

pub struct SccRef<'a, M: AccessMesadata> {
    scc: &'a Scc,
    autorouter: &'a Autorouter<M>,
}

impl<'a, M: AccessMesadata> SccRef<'a, M> {
    pub fn new(scc: &'a Scc, autorouter: &'a Autorouter<M>) -> Self {
        Self { scc, autorouter }
    }

    pub fn contains(&self, ratline: RatlineUid) -> bool {
        self.scc.node_indices().contains(
            &self
                .autorouter
                .ratsnests()
                .on_principal_layer(ratline.principal_layer)
                .graph()
                .edge_endpoints(ratline.index)
                .unwrap()
                .0,
        ) && self.scc.node_indices().contains(
            &self
                .autorouter
                .ratsnests()
                .on_principal_layer(ratline.principal_layer)
                .graph()
                .edge_endpoints(ratline.index)
                .unwrap()
                .1,
        )
    }
}
