// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    board::{
        Board,
        interactors::{SelectionCombineMode, SelectionContainMode},
        selections::PersistableSelection,
    },
    interactor::Interactor,
    layout::LayerId,
    rect::Rect3,
    vector::{Vector2, Vector3},
};

#[derive(Clone, Constructor, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DragSelectOptions {
    combine: SelectionCombineMode,
    contain: SelectionContainMode,
}

#[derive(Clone, Debug, Eq, Getters, PartialEq)]
pub struct DragSelectInteractor {
    origin: Vector2<i64>,
    layer: LayerId,
    original_selection: PersistableSelection,
    selection: PersistableSelection,
    options: DragSelectOptions,
}

impl DragSelectInteractor {
    pub fn new(
        origin: Vector2<i64>,
        layer: LayerId,
        original_selection: PersistableSelection,
        options: DragSelectOptions,
    ) -> Self {
        Self {
            origin,
            layer,
            original_selection,
            selection: PersistableSelection::new(),
            options,
        }
    }
}

impl Interactor for DragSelectInteractor {
    fn hold(&mut self, board: &mut Board, _layer: LayerId, pointer: Vector2<i64>) {
        self.selection = PersistableSelection::new();

        let rect = Rect3::new(
            Vector3::new(self.origin.x, self.origin.y, self.layer.index() as i64),
            Vector3::new(pointer.x, pointer.y, self.layer.index() as i64),
        );

        let all_belong_to_pins = match self.options.contain {
            SelectionContainMode::Crossing => {
                board
                    .layout()
                    .locate_joints_prefer_layer_intersecting_rect(rect)
                    .all(|joint_id| board.layout().joint(joint_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_segs_prefer_layer_intersecting_rect(rect)
                        .all(|seg_id| board.layout().seg(seg_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_vias_prefer_layer_intersecting_rect(rect)
                        .all(|via_id| board.layout().via(via_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_polys_prefer_layer_intersecting_rect(rect)
                        .all(|poly_id| board.layout().poly(poly_id).spec.pin.is_some())
            }
            SelectionContainMode::Window => {
                board
                    .layout()
                    .locate_joints_prefer_layer_inside_rect(rect)
                    .all(|joint_id| board.layout().joint(joint_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_segs_prefer_layer_inside_rect(rect)
                        .all(|seg_id| board.layout().seg(seg_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_vias_prefer_layer_inside_rect(rect)
                        .all(|via_id| board.layout().via(via_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_polys_prefer_layer_inside_rect(rect)
                        .all(|poly_id| board.layout().poly(poly_id).spec.pin.is_some())
            }
        };

        if !all_belong_to_pins {
            match self.options.contain {
                SelectionContainMode::Crossing => {
                    self.selection
                        .components
                        .add(board.locate_components_prefer_layer_intersecting_rect(rect));
                }
                SelectionContainMode::Window => {
                    self.selection
                        .components
                        .add(board.locate_components_prefer_layer_inside_rect(rect));
                }
            }

            match self.options.contain {
                SelectionContainMode::Crossing => {
                    self.selection
                        .nets
                        .add(board.locate_nets_prefer_layer_intersecting_rect(rect));
                }
                SelectionContainMode::Window => {
                    self.selection
                        .nets
                        .add(board.locate_nets_prefer_layer_inside_rect(rect));
                }
            }
        }

        match self.options.contain {
            SelectionContainMode::Crossing => {
                self.selection
                    .pins
                    .add(board.locate_pins_prefer_layer_intersecting_rect(rect));
            }
            SelectionContainMode::Window => {
                self.selection
                    .pins
                    .add(board.locate_pins_prefer_layer_inside_rect(rect));
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
                    .toggle(self.selection.components.0.iter().cloned());
                combined_selection
                    .nets
                    .toggle(self.selection.nets.0.iter().cloned());
                combined_selection
                    .pins
                    .toggle(self.selection.pins.0.iter().cloned());
            }
        }

        self.selection = combined_selection;
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        self.hold(board, layer, pointer);
    }

    fn abort(&mut self, _board: &mut Board) {
        self.selection = self.original_selection.clone();
    }
}
