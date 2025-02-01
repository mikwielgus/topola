// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use topola::board::{AccessMesadata, Board};

pub struct AppearancePanel {
    // TODO:
    // In1.Cu shall be #7fc87f (#d5ecd5 when selected).
    // In2.Cu shall be #ce7d2c (#e8c39e when selected).
    pub visible: Box<[bool]>,
}

impl AppearancePanel {
    pub fn new(board: &Board<impl AccessMesadata>) -> Self {
        let layer_count = board.layout().drawing().layer_count();
        let visible = std::iter::repeat(true)
            .take(layer_count)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self { visible }
    }

    pub fn update(&mut self, ctx: &egui::Context, board: &Board<impl AccessMesadata>) {
        egui::SidePanel::right("right_side_panel").show(ctx, |ui| {
            ui.label("Layers");

            for (layer, visible) in self.visible.iter_mut().enumerate() {
                let layername = board
                    .layout()
                    .drawing()
                    .rules()
                    .layer_layername(layer)
                    .unwrap_or("Unnamed layer");

                ui.checkbox(visible, layername);
            }
        });
    }
}
