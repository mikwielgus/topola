// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::Getters;
use petgraph::unionfind::UnionFind;
use specctra_core::rules::AccessRules;

use crate::{
    drawing::{graph::PrimitiveIndex, primitive::GetJoints},
    graph::GetIndex,
    layout::Layout,
};

#[derive(Clone, Getters)]
pub struct Conncomps {
    unionfind: UnionFind<usize>,
}

impl Conncomps {
    pub fn new(layout: &Layout<impl AccessRules>) -> Self {
        let mut unionfind = UnionFind::new(layout.drawing().geometry().dot_index_bound());

        for primitive in layout.drawing().primitive_nodes() {
            match primitive {
                PrimitiveIndex::FixedSeg(seg) => {
                    let joints = layout.drawing().primitive(seg).joints();
                    unionfind.union(joints.0.index(), joints.1.index());
                }
                PrimitiveIndex::LoneLooseSeg(seg) => {
                    let joints = layout.drawing().primitive(seg).joints();
                    unionfind.union(joints.0.index(), joints.1.index());
                }
                PrimitiveIndex::SeqLooseSeg(seg) => {
                    let joints = layout.drawing().primitive(seg).joints();
                    unionfind.union(joints.0.index(), joints.1.index());
                }
                PrimitiveIndex::FixedBend(bend) => {
                    let joints = layout.drawing().primitive(bend).joints();
                    unionfind.union(joints.0.index(), joints.1.index());
                }
                PrimitiveIndex::LooseBend(bend) => {
                    let joints = layout.drawing().primitive(bend).joints();
                    unionfind.union(joints.0.index(), joints.1.index());
                }
                _ => (),
            }
        }

        Self { unionfind }
    }
}
