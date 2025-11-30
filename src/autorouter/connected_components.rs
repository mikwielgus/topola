// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
use std::collections::BTreeSet;

use derive_getters::Getters;
use petgraph::unionfind::UnionFind;
use specctra_core::mesadata::AccessMesadata;

use crate::{
    board::Board,
    drawing::{dot::FixedDotIndex, graph::PrimitiveIndex, primitive::GetJoints},
    geometry::GenericNode,
    graph::GetIndex,
};

#[derive(Clone, Getters)]
pub struct ConnectedComponents {
    unionfind: UnionFind<usize>,
}

impl ConnectedComponents {
    pub fn new(board: &Board<impl AccessMesadata>) -> Self {
        let mut dot_unionfind =
            UnionFind::new(board.layout().drawing().geometry().dot_index_bound());

        for node in board.layout().drawing().primitive_nodes() {
            Self::unionize_primitive_endpoint_dots(board, &mut dot_unionfind, node);
        }

        // Pins can have padstacks that span multiple layers. To account for
        // that, we have another loop to go over all the pins and connect all
        // their primitives.
        for pinname in board.pinnames() {
            Self::unionize_pin(board, &mut dot_unionfind, pinname);
        }

        Self {
            unionfind: dot_unionfind,
        }
    }

    pub fn new_with_principal_layer(
        board: &Board<impl AccessMesadata>,
        principal_layer: usize,
    ) -> Self {
        let mut principally_visited_pins = BTreeSet::new();
        let mut unionfind = UnionFind::new(board.layout().drawing().geometry().dot_index_bound());

        for node in board
            .layout()
            .drawing()
            .layer_primitive_nodes(principal_layer)
        {
            Self::unionize_primitive_endpoint_dots(board, &mut unionfind, node);

            if let Some(pinname) = board.node_pinname(&GenericNode::Primitive(node)) {
                principally_visited_pins.insert(pinname.clone());
            }
        }

        // Pins can have padstacks that span multiple layers. To account for
        // that, we have another loop to go over all the pins and connect all
        // their primitives.
        for pinname in board.pinnames() {
            if principally_visited_pins.contains(pinname) {
                Self::unionize_pin(board, &mut unionfind, pinname);
            }
        }

        Self { unionfind }
    }

    fn unionize_primitive_endpoint_dots(
        board: &Board<impl AccessMesadata>,
        dot_unionfind: &mut UnionFind<usize>,
        primitive: PrimitiveIndex,
    ) {
        match primitive {
            PrimitiveIndex::FixedSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(joints.0.index(), joints.1.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.0);
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.1);
            }
            PrimitiveIndex::LoneLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::SeqLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::FixedBend(bend) => {
                let joints = board.layout().drawing().primitive_ref(bend).joints();
                dot_unionfind.union(joints.0.index(), joints.1.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.0);
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.1);
            }
            PrimitiveIndex::LooseBend(bend) => {
                let joints = board.layout().drawing().primitive_ref(bend).joints();
                dot_unionfind.union(joints.0.index(), joints.1.index());
            }
            _ => (),
        }
    }

    fn unionize_pin(
        board: &Board<impl AccessMesadata>,
        dot_unionfind: &mut UnionFind<usize>,
        pinname: &str,
    ) {
        let mut iter = board.pinname_nodes(pinname);
        let Some(first_fixed_dot) = iter.find_map(|node| {
            if let GenericNode::Primitive(PrimitiveIndex::FixedDot(first_fixed_dot)) = node {
                Some(first_fixed_dot)
            } else {
                None
            }
        }) else {
            return;
        };

        for node in board.pinname_nodes(pinname) {
            if let GenericNode::Primitive(primitive) = node {
                Self::unionize_to_common(board, dot_unionfind, primitive, first_fixed_dot);
            }
        }
    }

    fn unionize_to_common(
        board: &Board<impl AccessMesadata>,
        dot_unionfind: &mut UnionFind<usize>,
        primitive: PrimitiveIndex,
        common: FixedDotIndex,
    ) {
        match primitive {
            PrimitiveIndex::FixedDot(dot) => {
                dot_unionfind.union(common.index(), dot.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, dot);
            }
            PrimitiveIndex::LooseDot(dot) => {
                dot_unionfind.union(common.index(), dot.index());
            }
            PrimitiveIndex::FixedSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(common.index(), joints.0.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.0);
                dot_unionfind.union(common.index(), joints.1.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.1);
            }
            PrimitiveIndex::LoneLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(common.index(), joints.0.index());
                dot_unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::SeqLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive_ref(seg).joints();
                dot_unionfind.union(common.index(), joints.0.index());
                dot_unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::FixedBend(bend) => {
                let joints = board.layout().drawing().primitive_ref(bend).joints();
                dot_unionfind.union(common.index(), joints.0.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.0);
                dot_unionfind.union(common.index(), joints.1.index());
                Self::unionize_fixed_dot_via(board, dot_unionfind, joints.1);
            }
            PrimitiveIndex::LooseBend(bend) => {
                let joints = board.layout().drawing().primitive_ref(bend).joints();
                dot_unionfind.union(common.index(), joints.0.index());
                dot_unionfind.union(common.index(), joints.1.index());
            }
            _ => (),
        }
    }

    fn unionize_fixed_dot_via(
        board: &Board<impl AccessMesadata>,
        dot_unionfind: &mut UnionFind<usize>,
        dot: FixedDotIndex,
    ) {
        if let Some(via) = board.layout().fixed_dot_via(dot) {
            for via_dot in board.layout().via_ref(via).dots() {
                dot_unionfind.union(dot.index(), via_dot.index());
            }
        }
    }
}
