// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    Vector2, Vector3,
    board::{
        Board,
        interactors::{
            DragSelectionInteractor, DragSelectionOptions, InteractiveInput, SelectionCombineMode,
            SelectionContainMode,
        },
        selections::PersistableSelection,
    },
    layout::LayerId,
};

#[derive(Clone, Debug, Eq, Getters, PartialEq)]
pub struct SelectionInteractor {
    origin: Vector2<i64>,
    original_selection: PersistableSelection,
    selection: PersistableSelection,
    combine: SelectionCombineMode,
}

impl SelectionInteractor {
    pub fn new(
        origin: Vector2<i64>,
        original_selection: PersistableSelection,
        combine: SelectionCombineMode,
    ) -> Self {
        Self {
            origin,
            original_selection,
            selection: PersistableSelection::new(),
            combine,
        }
    }

    pub fn update(
        &mut self,
        board: &Board,
        layer: LayerId,
        input: InteractiveInput,
    ) -> Option<PersistableSelection> {
        if input.cancel {
            self.selection = self.original_selection.clone();
            return Some(self.selection.clone());
        }

        if input.release && input.pointer == self.origin {
            let mut selection = self.original_selection.clone();

            // Pins have intentional precedence over nets and components.
            if let Some(pin_selector) = board.locate_pin_at_point(Vector3::new(
                input.pointer.x,
                input.pointer.y,
                layer.index() as i64,
            )) {
                selection.pins.xor(std::iter::once(pin_selector));
            } else if let Some(net_selector) = board.locate_net_at_point(Vector3::new(
                input.pointer.x,
                input.pointer.y,
                layer.index() as i64,
            )) {
                selection.nets.xor(std::iter::once(net_selector));
            } else if let Some(component_selector) = board.locate_component_at_point(Vector3::new(
                input.pointer.x,
                input.pointer.y,
                layer.index() as i64,
            )) {
                selection
                    .components
                    .xor(std::iter::once(component_selector));
            }

            self.selection = selection.clone();
            return Some(selection);
        }

        let contain = if input.pointer.x >= self.origin.x {
            SelectionContainMode::Window
        } else {
            SelectionContainMode::Crossing
        };

        let options = DragSelectionOptions::new(self.combine.clone(), contain);
        let mut drag_selection_interactor =
            DragSelectionInteractor::new(self.origin, self.original_selection.clone(), options);
        let selection = drag_selection_interactor.update(board, input)?;

        self.selection = selection.clone();
        Some(selection)
    }
}
