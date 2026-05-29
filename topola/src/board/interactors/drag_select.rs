// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::{
    Rect3, Vector2, Vector3,
    board::{
        Board,
        interactors::{SelectionCombineMode, SelectionContainMode},
        selections::PersistableSelection,
    },
    layout::LayerId,
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

    pub fn abort(&mut self) {
        self.selection = self.original_selection.clone();
    }

    pub fn hold(&mut self, board: &Board, pointer: Vector2<i64>) {
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
                        .locate_segments_prefer_layer_intersecting_rect(rect)
                        .all(|segment_id| board.layout().segment(segment_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_vias_prefer_layer_intersecting_rect(rect)
                        .all(|via_id| board.layout().via(via_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_polygons_prefer_layer_intersecting_rect(rect)
                        .all(|polygon_id| board.layout().polygon(polygon_id).pin.is_some())
            }
            SelectionContainMode::Window => {
                board
                    .layout()
                    .locate_joints_prefer_layer_inside_rect(rect)
                    .all(|joint_id| board.layout().joint(joint_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_segments_prefer_layer_inside_rect(rect)
                        .all(|segment_id| board.layout().segment(segment_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_vias_prefer_layer_inside_rect(rect)
                        .all(|via_id| board.layout().via(via_id).spec.pin.is_some())
                    && board
                        .layout()
                        .locate_polygons_prefer_layer_inside_rect(rect)
                        .all(|polygon_id| board.layout().polygon(polygon_id).pin.is_some())
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
    }
}
