// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    Rect2, Vector2,
    board::{Board, interactors::InteractiveInput, selections::PersistableSelection},
    layout::LayerId,
};

#[derive(Clone, Debug, Eq, Getters, Ord, PartialEq, PartialOrd)]
pub struct CrossingDragSelectionInteractor {
    origin: Vector2<i64>,
    selection: PersistableSelection,
}

impl CrossingDragSelectionInteractor {
    pub fn new(origin: Vector2<i64>) -> Self {
        Self {
            origin,
            selection: PersistableSelection::new(),
        }
    }

    pub fn update(&mut self, board: &Board, input: InteractiveInput) {
        let rect = Rect2::new(self.origin, input.pointer);

        self.selection = PersistableSelection::new();

        for layer_index in 0..*board.layout().layer_count() {
            let layer = LayerId::new(layer_index);

            for selector in board.locate_components_intersecting_rect(layer, rect) {
                self.selection.components.0.insert(selector);
            }

            for selector in board.locate_pins_intersecting_rect(layer, rect) {
                self.selection.pins.0.insert(selector);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Getters, Ord, PartialEq, PartialOrd)]
pub struct WindowDragSelectionInteractor {
    origin: Vector2<i64>,
    selection: PersistableSelection,
}

impl WindowDragSelectionInteractor {
    pub fn new(origin: Vector2<i64>) -> Self {
        Self {
            origin,
            selection: PersistableSelection::new(),
        }
    }

    pub fn update(&mut self, board: &Board, input: InteractiveInput) {
        let rect = Rect2::new(self.origin, input.pointer);

        self.selection = PersistableSelection::new();

        for layer_index in 0..*board.layout().layer_count() {
            let layer = LayerId::new(layer_index);

            for selector in board.locate_components_inside_rect(layer, rect) {
                self.selection.components.0.insert(selector);
            }

            for selector in board.locate_pins_inside_rect(layer, rect) {
                self.selection.pins.0.insert(selector);
            }
        }
    }
}
