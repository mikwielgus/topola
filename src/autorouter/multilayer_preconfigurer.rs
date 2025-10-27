// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use derive_getters::Getters;
use specctra_core::mesadata::AccessMesadata;

use crate::{
    autorouter::{anterouter::AnterouterPlan, ratline::RatlineUid, Autorouter},
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
                .map(|(_i, ratline)| (*ratline, ratline.ref_(autorouter).preferred_layer()))
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
            static_terminating_dot_map: BTreeMap::new(),
        };

        for ratline in ratlines {
            for layer in 0..autorouter.board().layout().drawing().layer_count() {
                if let Some(static_terminating_dot) = Self::find_static_terminating_dot(
                    autorouter,
                    ratline.ref_(autorouter).endpoint_dots().0,
                    layer,
                ) {
                    plan.static_terminating_dot_map.insert(
                        (*ratline, ratline.ref_(autorouter).endpoint_dots().0, layer),
                        static_terminating_dot,
                    );
                }

                if let Some(static_terminating_dot) = Self::find_static_terminating_dot(
                    autorouter,
                    ratline.ref_(autorouter).endpoint_dots().1,
                    layer,
                ) {
                    plan.static_terminating_dot_map.insert(
                        (*ratline, ratline.ref_(autorouter).endpoint_dots().1, layer),
                        static_terminating_dot,
                    );
                }
            }
        }

        Self { plan }
    }

    fn find_static_terminating_dot(
        autorouter: &Autorouter<impl AccessMesadata>,
        ratline_endpoint_dot: FixedDotIndex,
        layer: usize,
    ) -> Option<FixedDotIndex> {
        let pinname = autorouter
            .board()
            .node_pinname(&GenericNode::Primitive(ratline_endpoint_dot.into()))
            .unwrap();

        autorouter.board().pinname_nodes(pinname).find_map(|node| {
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
    }
}
