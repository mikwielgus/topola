// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::debug_ensures;
use petgraph::data::DataMap;

use crate::{
    drawing::{
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        head::{BareHead, CaneHead, Head},
        rules::AccessRules,
    },
    layout::{Layout, LayoutEdit},
};

use super::{
    draw::Draw,
    navcorder::{Navcorder, NavcorderException},
    navmesh::{BinavnodeNodeIndex, Navmesh, NavnodeIndex},
};

/// The `Navcord` is a data structure that holds the movable non-borrowing data
/// of the currently running routing process.
///
/// Note that this data structure is not a stepper, since steppers always
/// progress linearly, whereas `Navcord` branches out to different states
/// depending on the value of the `to` argument of the `.step_to(...)` method.
///
/// The name "navcord" is a shortening of "navigation cord", by analogy to
/// "navmesh" being a shortening of "navigation mesh".
#[derive(Debug)]
pub struct Navcord {
    /// The layout edit which we are currently recording.
    pub recorder: LayoutEdit,
    /// The currently attempted path.
    pub path: Vec<NavnodeIndex>,
    /// The head of the currently routed band.
    pub head: Head,
    /// If the band is finished, stores the termseg that was used to finish it.
    pub final_termseg: Option<BandTermsegIndex>,
    /// The width of the currently routed band.
    pub width: f64,
}

impl Navcord {
    /// Create a new navcord.
    pub fn new(
        recorder: LayoutEdit,
        source: FixedDotIndex,
        source_navnode: NavnodeIndex,
        width: f64,
    ) -> Navcord {
        Self {
            recorder,
            path: vec![source_navnode],
            head: BareHead { face: source }.into(),
            final_termseg: None,
            width,
        }
    }

    /// From the current head `head` wrap a new head around the navnode `around`.
    fn wrap(
        &mut self,
        layout: &mut Layout<impl AccessRules>,
        navmesh: &Navmesh,
        head: Head,
        around: NavnodeIndex,
    ) -> Result<CaneHead, NavcorderException> {
        let around_node_weight = navmesh.node_weight(around).unwrap();
        let sense = around_node_weight
            .maybe_sense
            .ok_or(NavcorderException::CannotWrap)?;

        match around_node_weight.node {
            BinavnodeNodeIndex::FixedDot(dot) => {
                layout.cane_around_dot(&mut self.recorder, head, dot, sense, self.width)
            }
            BinavnodeNodeIndex::FixedBend(fixed_bend) => layout.cane_around_bend(
                &mut self.recorder,
                head,
                fixed_bend.into(),
                sense,
                self.width,
            ),
            BinavnodeNodeIndex::LooseBend(loose_bend) => layout.cane_around_bend(
                &mut self.recorder,
                head,
                loose_bend.into(),
                sense,
                self.width,
            ),
        }
        .map_err(NavcorderException::CannotDraw)
    }

    /// Advance the navcord and the currently routed band by one step to the
    /// navnode `to`.
    #[debug_ensures(ret.is_ok() -> self.path.len() == old(self.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> self.path.len() == old(self.path.len()))]
    pub fn step_to<R: AccessRules>(
        &mut self,
        layout: &mut Layout<R>,
        navmesh: &Navmesh,
        to: NavnodeIndex,
    ) -> Result<(), NavcorderException> {
        if to == navmesh.destination_navnode() {
            let to_node_weight = navmesh.node_weight(to).unwrap();
            let BinavnodeNodeIndex::FixedDot(to_dot) = to_node_weight.node else {
                unreachable!();
            };

            self.final_termseg = Some(layout.finish(navmesh, self, to_dot)?);

            // NOTE: We don't update the head here because there is currently
            // no head variant that consists only of a seg, and I'm not sure if
            // there should be one.
        } else {
            self.head = self.wrap(layout, navmesh, self.head, to)?.into();
        }

        // Now that the new part of the trace has been created, push the
        // navnode `to` onto the currently attempted path to start from it
        // on the next `.step_to(...)` call or retreat from it later using
        // `.step_back(...)`.
        self.path.push(to);
        Ok(())
    }

    /// Retreat the navcord and the currently routed band by one step.
    #[debug_ensures(self.path.len() == old(self.path.len() - 1))]
    pub fn step_back<R: AccessRules>(
        &mut self,
        layout: &mut Layout<R>,
    ) -> Result<(), NavcorderException> {
        if let Some(final_termseg) = self.final_termseg {
            layout.remove_termseg(&mut self.recorder, final_termseg);
            self.final_termseg = None;
        } else {
            if let Head::Cane(head) = self.head {
                self.head = layout.undo_cane(&mut self.recorder, head).unwrap();
            } else {
                // "can't unwrap"
                return Err(NavcorderException::CannotWrap);
            }
        }

        // Now that the last head of the currently routed band was deleted, pop
        // the last navnode from the currently attempted path so that it is up
        // to date.
        self.path.pop();
        Ok(())
    }
}
