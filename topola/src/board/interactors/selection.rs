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

    pub fn update(&mut self, board: &Board, layer: LayerId, input: InteractiveInput) {
        if input.cancel {
            self.selection = self.original_selection.clone();
            return;
        }

        if input.release && input.pointer == self.origin {
            let mut selection = self.original_selection.clone();
            let point = Vector3::new(input.pointer.x, input.pointer.y, layer.index() as i64);

            // Pins have intentional precedence over nets and components.
            if let Some(pin_selector) = board.locate_pins_prefer_layer_at_point(point).next() {
                selection.pins.xor(std::iter::once(pin_selector));
            } else if let Some(net_selector) = board.locate_nets_prefer_layer_at_point(point).next() {
                selection.nets.xor(std::iter::once(net_selector));
            } else if let Some(component_selector) =
                board.locate_components_prefer_layer_at_point(point).next()
            {
                selection
                    .components
                    .xor(std::iter::once(component_selector));
            }

            self.selection = selection;
            return;
        }

        let contain = if input.pointer.x >= self.origin.x {
            SelectionContainMode::Window
        } else {
            SelectionContainMode::Crossing
        };

        let options = DragSelectionOptions::new(self.combine.clone(), contain);
        let mut drag_selection_interactor = DragSelectionInteractor::new(
            self.origin,
            layer,
            self.original_selection.clone(),
            options,
        );

        drag_selection_interactor.update(board, input);
        self.selection = drag_selection_interactor.selection().clone();
    }
}
