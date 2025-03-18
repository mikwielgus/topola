// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::debug_ensures;
use petgraph::data::DataMap;

use crate::{
    drawing::{
        dot::FixedDotIndex,
        head::{BareHead, CaneHead, Head},
        rules::AccessRules,
    },
    layout::{Layout, LayoutEdit},
};

use super::{
    draw::Draw,
    navcorder::NavcorderException,
    navmesh::{BinavvertexNodeIndex, Navmesh, NavvertexIndex},
};

/// The navcord (stepper) is a structure that holds the movable non-borrowing
/// data of the currently running routing process.
///
/// The name "navcord" is a shortening of "navigation cord", by analogy to
/// "navmesh" being a shortening of "navigation mesh".
#[derive(Debug)]
pub struct NavcordStepper {
    pub recorder: LayoutEdit,
    /// The currently attempted path.
    pub path: Vec<NavvertexIndex>,
    /// The head of the routed band.
    pub head: Head,
    /// The width of the routed band.
    pub width: f64,
}

impl NavcordStepper {
    /// Creates a new navcord.
    pub fn new(
        recorder: LayoutEdit,
        source: FixedDotIndex,
        source_navvertex: NavvertexIndex,
        width: f64,
    ) -> NavcordStepper {
        Self {
            recorder,
            path: vec![source_navvertex],
            head: BareHead { face: source }.into(),
            width,
        }
    }

    fn wrap(
        &mut self,
        layout: &mut Layout<impl AccessRules>,
        navmesh: &Navmesh,
        head: Head,
        around: NavvertexIndex,
    ) -> Result<CaneHead, NavcorderException> {
        let around_node_weight = navmesh.node_weight(around).unwrap();
        let cw = around_node_weight
            .maybe_cw
            .ok_or(NavcorderException::CannotWrap)?;

        match around_node_weight.node {
            BinavvertexNodeIndex::FixedDot(dot) => {
                layout.cane_around_dot(&mut self.recorder, head, dot, cw, self.width)
            }
            BinavvertexNodeIndex::FixedBend(fixed_bend) => {
                layout.cane_around_bend(&mut self.recorder, head, fixed_bend.into(), cw, self.width)
            }
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                layout.cane_around_bend(&mut self.recorder, head, loose_bend.into(), cw, self.width)
            }
        }
        .map_err(NavcorderException::CannotDraw)
    }

    #[debug_ensures(ret.is_ok() -> matches!(self.head, Head::Cane(..)))]
    #[debug_ensures(ret.is_ok() -> self.path.len() == old(self.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> self.path.len() == old(self.path.len()))]
    pub fn step<R: AccessRules>(
        &mut self,
        layout: &mut Layout<R>,
        navmesh: &Navmesh,
        to: NavvertexIndex,
    ) -> Result<(), NavcorderException> {
        self.head = self.wrap(layout, navmesh, self.head, to)?.into();
        self.path.push(to);

        Ok(())
    }

    #[debug_ensures(self.path.len() == old(self.path.len() - 1))]
    pub fn step_back<R: AccessRules>(
        &mut self,
        layout: &mut Layout<R>,
    ) -> Result<(), NavcorderException> {
        if let Head::Cane(head) = self.head {
            self.head = layout.undo_cane(&mut self.recorder, head).unwrap();
        } else {
            // "can't unwrap"
            return Err(NavcorderException::CannotWrap);
        }

        self.path.pop();
        Ok(())
    }
}
