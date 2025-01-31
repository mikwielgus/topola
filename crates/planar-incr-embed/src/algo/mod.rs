// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! planar embedding / plane graph algorithms

pub mod pmg_astar;

use crate::{navmesh::NavmeshRef, NavmeshBase, NavmeshIndex};
use alloc::{collections::BTreeSet, vec::Vec};

/// The goal of one search iteration (a single path to be etched / embedded)
#[derive(Clone, Debug)]
pub struct Goal<PNI, EP> {
    pub source: PNI,
    pub target: BTreeSet<PNI>,
    pub label: EP,
}

#[derive(Clone, Debug)]
pub struct TargetNetwork<Scalar>(Vec<spade::Point2<Scalar>>);

#[derive(Clone, Debug)]
pub struct PreparedGoal<B: NavmeshBase> {
    pub source: B::PrimalNodeIndex,
    pub target: BTreeSet<B::PrimalNodeIndex>,
    pub label: B::EtchedPath,

    pub target_network: TargetNetwork<B::Scalar>,
    pub minimal_costs: B::Scalar,
}

impl<PNI: Clone + Eq + Ord, EP: Clone + Eq> Goal<PNI, EP> {
    pub fn target_network<B: NavmeshBase<PrimalNodeIndex = PNI, EtchedPath = EP>>(
        &self,
        navmesh: NavmeshRef<'_, B>,
    ) -> TargetNetwork<B::Scalar> {
        TargetNetwork(
            self.target
                .iter()
                .cloned()
                .map(|i| &navmesh.node_data(&NavmeshIndex::Primal(i)).unwrap().pos)
                .cloned()
                .collect(),
        )
    }

    pub fn estimate_costs_for_source<B: NavmeshBase<PrimalNodeIndex = PNI, EtchedPath = EP>>(
        &self,
        navmesh: NavmeshRef<'_, B>,
        source: &NavmeshIndex<PNI>,
    ) -> B::Scalar
    where
        B::Scalar: num_traits::Float,
    {
        self.target_network(navmesh)
            .estimate_costs(&navmesh.node_data(source).unwrap().pos)
    }

    pub fn estimate_costs<B: NavmeshBase<PrimalNodeIndex = PNI, EtchedPath = EP>>(
        &self,
        navmesh: NavmeshRef<'_, B>,
    ) -> B::Scalar
    where
        B::Scalar: num_traits::Float,
    {
        self.estimate_costs_for_source(navmesh, &NavmeshIndex::Primal(self.source.clone()))
    }

    pub fn prepare<B: NavmeshBase<PrimalNodeIndex = PNI, EtchedPath = EP>>(
        self,
        navmesh: NavmeshRef<'_, B>,
    ) -> PreparedGoal<B>
    where
        B::Scalar: num_traits::Float,
    {
        let target_network = self.target_network(navmesh);
        let minimal_costs = target_network.estimate_costs(
            &navmesh
                .node_data(&NavmeshIndex::Primal(self.source.clone()))
                .unwrap()
                .pos,
        );

        PreparedGoal {
            source: self.source,
            target: self.target,
            label: self.label,

            target_network,
            minimal_costs,
        }
    }
}

impl<Scalar: num_traits::Float> TargetNetwork<Scalar> {
    pub fn estimate_costs(&self, source_pos: &spade::Point2<Scalar>) -> Scalar {
        self.0
            .iter()
            .map(|i| crate::utils::euclidean_distance(source_pos, i))
            .reduce(Scalar::min)
            .unwrap()
    }
}

impl<B: NavmeshBase> PreparedGoal<B>
where
    B::Scalar: num_traits::Float,
{
    pub fn estimate_costs_for_source<CT>(
        &self,
        navmesh: NavmeshRef<'_, B>,
        source: &NavmeshIndex<B::PrimalNodeIndex>,
    ) -> B::Scalar {
        self.target_network
            .estimate_costs(&navmesh.node_data(source).unwrap().pos)
    }
}
