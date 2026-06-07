// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::Board, layout::compounds::ComponentId, selections::ComponentSelection, vector::Vector2,
};

impl Board {
    pub fn move_components_by(&mut self, selection: ComponentSelection, translation: Vector2<i64>) {
        self.move_resolved_components_by(
            &self.resolve_components(selection).collect::<Vec<_>>(),
            translation,
        );
    }

    pub fn move_resolved_components_by(
        &mut self,
        selection: &[ComponentId],
        translation: Vector2<i64>,
    ) {
        crate::profile_function!();
        self.layout.move_components_by(selection, translation);
    }
}
