// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board, layout::compounds::ComponentId, rect::Rect2, selections::ComponentSelection,
};

impl Board {
    pub fn components_bbox2(&self, selection: ComponentSelection) -> Option<Rect2<i64>> {
        self.resolved_components_bbox2(&self.resolve_components(selection).collect::<Vec<_>>())
    }

    pub fn resolved_components_bbox2(&self, selection: &[ComponentId]) -> Option<Rect2<i64>> {
        self.layout.components_bbox2(selection.iter().copied())
    }
}
