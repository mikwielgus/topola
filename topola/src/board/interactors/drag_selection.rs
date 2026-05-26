// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    Rect3, Vector2, Vector3,
    board::{
        Board,
        interactors::{
            InteractiveInput, SelectionCombineMode, SelectionContainMode, SelectionOptions,
        },
        selections::PersistableSelection,
    },
};

#[derive(Clone, Debug, Eq, Getters, Ord, PartialEq, PartialOrd)]
pub struct DragSelectionInteractor {
    origin: Vector2<i64>,
    original_selection: PersistableSelection,
    selection: PersistableSelection,
    options: SelectionOptions,
}

impl DragSelectionInteractor {
    pub fn new(
        origin: Vector2<i64>,
        original_selection: PersistableSelection,
        options: SelectionOptions,
    ) -> Self {
        Self {
            origin,
            original_selection,
            selection: PersistableSelection::new(),
            options,
        }
    }

    pub fn update(
        &mut self,
        board: &Board,
        input: InteractiveInput,
    ) -> Option<PersistableSelection> {
        if input.cancel {
            self.selection = self.original_selection.clone();
            return Some(self.selection.clone());
        }

        self.selection = PersistableSelection::new();

        for layer_index in 0..*board.layout().layer_count() {
            let rect = Rect3::new(
                Vector3::new(self.origin.x, self.origin.y, layer_index as i64),
                Vector3::new(input.pointer.x, input.pointer.y, layer_index as i64),
            );

            match self.options.contain {
                SelectionContainMode::Crossing => {
                    self.selection
                        .components
                        .add(board.locate_components_intersecting_rect(rect));
                }
                SelectionContainMode::Window => {
                    self.selection
                        .components
                        .add(board.locate_components_inside_rect(rect));
                }
            }

            match self.options.contain {
                SelectionContainMode::Crossing => {
                    self.selection
                        .nets
                        .add(board.locate_nets_intersecting_rect(rect));
                }
                SelectionContainMode::Window => {
                    self.selection.nets.add(board.locate_nets_inside_rect(rect));
                }
            }

            match self.options.contain {
                SelectionContainMode::Crossing => {
                    self.selection
                        .pins
                        .add(board.locate_pins_intersecting_rect(rect));
                }
                SelectionContainMode::Window => {
                    self.selection.pins.add(board.locate_pins_inside_rect(rect));
                }
            }
        }

        // TODO: There's no need to clone on each update.
        let mut combined_selection = self.original_selection.clone();
        match self.options.combine {
            SelectionCombineMode::Replace => {
                combined_selection.components = self.selection.components.clone();
                combined_selection.nets = self.selection.nets.clone();
                combined_selection.pins = self.selection.pins.clone();
            }
            SelectionCombineMode::Additive => {
                combined_selection
                    .pins
                    .add(self.selection.pins.0.iter().cloned());
                combined_selection
                    .nets
                    .add(self.selection.nets.0.iter().cloned());
                combined_selection
                    .components
                    .add(self.selection.components.0.iter().cloned());
            }
            SelectionCombineMode::Subtractive => {
                combined_selection
                    .pins
                    .sub(self.selection.pins.0.iter().cloned());
                combined_selection
                    .nets
                    .sub(self.selection.nets.0.iter().cloned());
                combined_selection
                    .components
                    .sub(self.selection.components.0.iter().cloned());
            }
            SelectionCombineMode::Toggle => {
                combined_selection
                    .components
                    .xor(self.selection.components.0.iter().cloned());
                combined_selection
                    .nets
                    .xor(self.selection.nets.0.iter().cloned());
                combined_selection
                    .pins
                    .xor(self.selection.pins.0.iter().cloned());
            }
        }

        self.selection = combined_selection;
        Some(self.selection.clone())
    }
}
