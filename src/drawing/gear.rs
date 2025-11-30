// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use enum_dispatch::enum_dispatch;
use petgraph::visit::Walker;

use crate::{
    drawing::{
        bend::{BendIndex, FixedBendIndex, LooseBendIndex},
        dot::FixedDotIndex,
        graph::{MakePrimitiveRef, PrimitiveIndex},
        primitive::{FixedBendRef, FixedDotRef, LooseBendRef, PrimitiveRef},
        rules::AccessRules,
        Drawing,
    },
    graph::{GetIndex, MakeRef},
};

#[enum_dispatch(GetIndex, MakePrimitiveRef)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GearIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl<'a, CW: 'a, Cel: 'a, R: 'a> MakeRef<'a, Drawing<CW, Cel, R>> for GearIndex {
    type Output = GearRef<'a, CW, Cel, R>;
    fn ref_(&self, drawing: &'a Drawing<CW, Cel, R>) -> GearRef<'a, CW, Cel, R> {
        GearRef::<'a, CW, Cel, R>::new(*self, drawing)
    }
}

impl From<GearIndex> for PrimitiveIndex {
    fn from(wraparoundable: GearIndex) -> Self {
        match wraparoundable {
            GearIndex::FixedDot(dot) => PrimitiveIndex::FixedDot(dot),
            GearIndex::FixedBend(bend) => PrimitiveIndex::FixedBend(bend),
            GearIndex::LooseBend(bend) => PrimitiveIndex::LooseBend(bend),
        }
    }
}

impl From<BendIndex> for GearIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => GearIndex::FixedBend(bend),
            BendIndex::Loose(bend) => GearIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetOuterGears, WalkOutwards, GetDrawing, GetIndex)]
pub enum GearRef<'a, CW, Cel, R> {
    FixedDot(FixedDotRef<'a, CW, Cel, R>),
    FixedBend(FixedBendRef<'a, CW, Cel, R>),
    LooseBend(LooseBendRef<'a, CW, Cel, R>),
}

impl<'a, CW, Cel, R> GearRef<'a, CW, Cel, R> {
    pub fn new(index: GearIndex, drawing: &'a Drawing<CW, Cel, R>) -> Self {
        match index {
            GearIndex::FixedDot(dot) => drawing.primitive_ref(dot).into(),
            GearIndex::FixedBend(bend) => drawing.primitive_ref(bend).into(),
            GearIndex::LooseBend(bend) => drawing.primitive_ref(bend).into(),
        }
    }
}

#[enum_dispatch]
pub trait GetOuterGears {
    // TODO: This duplicates `.outers()` methods in some other places, we need
    // to merge them with this.
    // TODO: Use iterator instead of vec.
    fn outer_gears(&self) -> Vec<LooseBendIndex>;
}

#[enum_dispatch]
pub trait WalkOutwards {
    fn outwards(&self) -> DrawingOutwardWalker;
}

/// I found it easier to just duplicate `OutwardWalker<BI>` for `Drawing<...>`.
pub struct DrawingOutwardWalker {
    frontier: VecDeque<LooseBendIndex>,
}

impl DrawingOutwardWalker {
    pub fn new(initial_frontier: impl Iterator<Item = LooseBendIndex>) -> Self {
        let mut frontier = VecDeque::new();
        frontier.extend(initial_frontier);

        Self { frontier }
    }
}

impl<CW: Clone, Cel: Copy, R: AccessRules> Walker<&Drawing<CW, Cel, R>> for DrawingOutwardWalker {
    type Item = LooseBendIndex;

    fn walk_next(&mut self, drawing: &Drawing<CW, Cel, R>) -> Option<Self::Item> {
        let front = self.frontier.pop_front()?;
        self.frontier.extend(drawing.primitive_ref(front).outers());

        Some(front)
    }
}
