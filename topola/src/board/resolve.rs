// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{Board, selections::ComponentSelection},
    layout::compounds::ComponentId,
};

impl Board {
    pub fn resolve_components(&self, selection: ComponentSelection) -> Vec<ComponentId> {
        selection
            .0
            .clone()
            .into_iter()
            .filter_map(|selector| self.component_id(&selector.component))
            .collect()
    }
}
