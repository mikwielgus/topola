// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    board::{
        Board,
        interactors::{
            DragSelectInteractor, DragSelectOptions, SelectionCombineMode, SelectionContainMode,
        },
        selections::PersistableSelection,
    },
    interactor::Interactor,
    layout::LayerId,
    vector::{Vector2, Vector3},
};

#[derive(Clone, Debug, Eq, Getters, PartialEq)]
pub struct SelectInteractor {
    origin: Vector2<i64>,
    original_selection: PersistableSelection,
    selection: PersistableSelection,
    combine: SelectionCombineMode,
}

impl SelectInteractor {
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
}

impl Interactor for SelectInteractor {
    fn delete(&mut self, _board: &mut Board) {}

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        let contain = if pointer.x >= self.origin.x {
            SelectionContainMode::Window
        } else {
            SelectionContainMode::Crossing
        };

        let options = DragSelectOptions::new(self.combine.clone(), contain);
        let mut drag_selection_interactor =
            DragSelectInteractor::new(self.origin, layer, self.original_selection.clone(), options);

        drag_selection_interactor.hold(board, layer, pointer);
        self.selection = drag_selection_interactor.selection().clone();
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        if pointer == self.origin {
            let mut selection = self.original_selection.clone();
            let point = Vector3::new(pointer.x, pointer.y, layer.index() as i64);

            // Pins have intentional precedence over nets and components.
            if let Some(pin_selector) = board.locate_pins_prefer_layer_at_point(point).next() {
                selection.pins.toggle(std::iter::once(pin_selector));
            } else if let Some(net_selector) = board.locate_nets_prefer_layer_at_point(point).next()
            {
                selection.nets.toggle(std::iter::once(net_selector));
            } else if let Some(component_selector) =
                board.locate_components_prefer_layer_at_point(point).next()
            {
                selection
                    .components
                    .toggle(std::iter::once(component_selector));
            }

            self.selection = selection;
            return;
        }

        self.hold(board, layer, pointer);
    }

    fn abort(&mut self, _board: &mut Board) {
        self.selection = self.original_selection.clone();
    }
}
