// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use egui::{widget_text::WidgetText, Context, Grid, ScrollArea, SidePanel};
use topola::board::{AccessMesadata, Board};

pub struct AppearancePanel {
    // TODO:
    // In1.Cu shall be #7fc87f (#d5ecd5 when selected).
    // In2.Cu shall be #ce7d2c (#e8c39e when selected).
    pub visible: Box<[bool]>,

    pub active_layer: Option<usize>,
}

impl AppearancePanel {
    pub fn new(board: &Board<impl AccessMesadata>) -> Self {
        let layer_count = board.layout().drawing().layer_count();
        let visible = core::iter::repeat(true)
            .take(layer_count)
            .collect::<Box<[_]>>();
        Self {
            visible,
            active_layer: Some(0),
        }
    }

    pub fn update(&mut self, ctx: &Context, board: &Board<impl AccessMesadata>) {
        SidePanel::right("appearance_panel").show(ctx, |ui| {
            ui.label("Layers");
            let row_height = ui.spacing().interact_size.y;
            ScrollArea::vertical().show_rows(
                ui,
                row_height,
                self.visible.len(),
                |ui, row_range| {
                    let start_row = row_range.start;
                    Grid::new("appearance_layers")
                        .min_col_width(ui.spacing().icon_width)
                        .num_columns(3)
                        .start_row(start_row)
                        .show(ui, |ui| {
                            for (layer, visible) in self.visible[row_range].iter_mut().enumerate() {
                                let layer = layer + start_row;
                                let layername =
                                    board.layout().drawing().rules().layer_layername(layer);

                                // unnamed layers can't be used for routing
                                if layername.is_some() {
                                    ui.radio_value(
                                        &mut self.active_layer,
                                        Some(layer),
                                        WidgetText::default(),
                                    );
                                } else {
                                    // dummy item to bump the grid
                                    ui.label("");
                                }
                                ui.checkbox(visible, WidgetText::default());
                                ui.label(
                                    layername
                                        .map(|i| i.to_string())
                                        .unwrap_or_else(|| format!("{} - Unnamed layer", layer)),
                                );
                                ui.end_row();
                            }
                        })
                },
            );
        });
    }
}
