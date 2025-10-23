// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use derive_getters::Getters;
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{
        anterouter::{AnterouterPlan, TerminatingScheme},
        ratline::RatlineUid,
        Autorouter,
    },
    drawing::{
        dot::FixedDotIndex,
        graph::{MakePrimitiveRef, PrimitiveIndex},
    },
    geometry::{GenericNode, GetLayer},
    graph::MakeRef,
};

#[derive(Clone, Debug)]
pub struct MultilayerAutoroutePreconfigurerInput {
    pub ratlines: BTreeSet<RatlineUid>,
}

#[derive(Getters)]
pub struct MultilayerPreconfigurer {
    plan: AnterouterPlan,
}

impl MultilayerPreconfigurer {
    pub fn new(
        autorouter: &Autorouter<impl AccessMesadata>,
        input: MultilayerAutoroutePreconfigurerInput,
    ) -> Self {
        Self::new_from_layer_map(
            autorouter,
            &input.ratlines,
            input
                .ratlines
                .iter()
                .enumerate()
                .map(|(i, ratline)| (*ratline, i % 2))
                .collect(),
        )
    }

    pub fn new_from_layer_map(
        autorouter: &Autorouter<impl AccessMesadata>,
        ratlines: &BTreeSet<RatlineUid>,
        layer_map: BTreeMap<RatlineUid, usize>,
    ) -> Self {
        let mut plan = AnterouterPlan {
            layer_map,
            ratline_terminating_schemes: BTreeMap::new(),
        };

        for ratline in ratlines {
            let layer = plan.layer_map[ratline];

            if let Some(terminating_scheme) = Self::determine_terminating_scheme(
                autorouter,
                ratline.ref_(autorouter).endpoint_dots().0,
                layer,
            ) {
                plan.ratline_terminating_schemes.insert(
                    (*ratline, ratline.ref_(autorouter).endpoint_dots().0),
                    terminating_scheme,
                );
            }

            if let Some(terminating_scheme) = Self::determine_terminating_scheme(
                autorouter,
                ratline.ref_(autorouter).endpoint_dots().1,
                layer,
            ) {
                plan.ratline_terminating_schemes.insert(
                    (*ratline, ratline.ref_(autorouter).endpoint_dots().1),
                    terminating_scheme,
                );
            }
        }

        Self { plan }
    }

    fn determine_terminating_scheme(
        autorouter: &Autorouter<impl AccessMesadata>,
        ratline_endpoint_dot: FixedDotIndex,
        layer: usize,
    ) -> Option<TerminatingScheme> {
        if layer
            == ratline_endpoint_dot
                .primitive_ref(autorouter.board().layout().drawing())
                .layer()
        {
            return None;
        }

        let pinname = autorouter
            .board()
            .node_pinname(&GenericNode::Primitive(ratline_endpoint_dot.into()))
            .unwrap();

        Some(
            autorouter
                .board()
                .pinname_nodes(pinname)
                .find_map(|node| {
                    if let GenericNode::Primitive(PrimitiveIndex::FixedDot(dot)) = node {
                        (layer
                            == dot
                                .primitive_ref(autorouter.board().layout().drawing())
                                .layer())
                        .then_some(dot)
                    } else {
                        None
                    }
                })
                .map_or(TerminatingScheme::Fanout, |dot| {
                    TerminatingScheme::ExistingFixedDot(dot)
                }),
        )
    }
}
