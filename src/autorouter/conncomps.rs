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
pub struct ConncompsWithPrincipalLayer {
    unionfind: UnionFind<usize>,
}

impl ConncompsWithPrincipalLayer {
    pub fn new(board: &Board<impl AccessMesadata>, principal_layer: usize) -> Self {
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

        /*for layer in 0..board.layout().drawing().layer_count() {
            if layer != principal_layer {
                for primitive in board.layout().drawing().layer_primitive_nodes(layer) {
                    if let Some(pinname) = board.node_pinname(&GenericNode::Primitive(primitive)) {
                        if !principally_visited_pins.contains(pinname) {
                            Self::unionize_by_primitive(board, &mut unionfind, primitive);
                        }
                    }
                }
            }
        }*/

        //TODO for pinname in board.pins() if !principally_visited_pins.contains(pinname) for node in board.pinname_nodes() unionize to first found element

        for pinname in board.pinnames() {
            if principally_visited_pins.contains(pinname) {
                let mut iter = board.pinname_nodes(pinname);
                let Some(first_fixed_dot) = iter.find_map(|node| {
                    if let GenericNode::Primitive(PrimitiveIndex::FixedDot(first_fixed_dot)) = node
                    {
                        Some(first_fixed_dot)
                    } else {
                        None
                    }
                }) else {
                    continue;
                };

                for node in board.pinname_nodes(pinname) {
                    if let GenericNode::Primitive(primitive) = node {
                        Self::unionize_to_common(board, &mut unionfind, primitive, first_fixed_dot);
                    }
                }
            }
        }

        Self { unionfind }
    }

    fn unionize_primitive_endpoint_dots(
        board: &Board<impl AccessMesadata>,
        unionfind: &mut UnionFind<usize>,
        primitive: PrimitiveIndex,
    ) {
        match primitive {
            PrimitiveIndex::FixedSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::LoneLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::SeqLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::FixedBend(bend) => {
                let joints = board.layout().drawing().primitive(bend).joints();
                unionfind.union(joints.0.index(), joints.1.index());
            }
            PrimitiveIndex::LooseBend(bend) => {
                let joints = board.layout().drawing().primitive(bend).joints();
                unionfind.union(joints.0.index(), joints.1.index());
            }
            _ => (),
        }
    }

    fn unionize_to_common(
        board: &Board<impl AccessMesadata>,
        unionfind: &mut UnionFind<usize>,
        primitive: PrimitiveIndex,
        common: FixedDotIndex,
    ) {
        match primitive {
            PrimitiveIndex::FixedDot(dot) => {
                unionfind.union(common.index(), dot.index());
            }
            PrimitiveIndex::LooseDot(dot) => {
                unionfind.union(common.index(), dot.index());
            }
            PrimitiveIndex::FixedSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(common.index(), joints.0.index());
                unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::LoneLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(common.index(), joints.0.index());
                unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::SeqLooseSeg(seg) => {
                let joints = board.layout().drawing().primitive(seg).joints();
                unionfind.union(common.index(), joints.0.index());
                unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::FixedBend(bend) => {
                let joints = board.layout().drawing().primitive(bend).joints();
                unionfind.union(common.index(), joints.0.index());
                unionfind.union(common.index(), joints.1.index());
            }
            PrimitiveIndex::LooseBend(bend) => {
                let joints = board.layout().drawing().primitive(bend).joints();
                unionfind.union(common.index(), joints.0.index());
                unionfind.union(common.index(), joints.1.index());
            }
            _ => (),
        }
    }
}
