// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::Pos2;

use crate::{viewport::Viewport, workspace::Workspace};

pub struct Displayer {}

impl Displayer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        self.display_layout(ctx, ui, /*menu_bar,*/ viewport, workspace);
    }

    pub fn display_layout(
        &mut self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        //menu_bar: &MenuBar,
        viewport: &Viewport,
        workspace: &Workspace,
    ) {
        ui.painter().line(
            workspace
                .board
                .layout()
                .boundary()
                .iter()
                .map(|p| egui::Pos2 {
                    x: p[0] as f32,
                    y: p[1] as f32,
                })
                .collect::<Vec<_>>(),
            egui::Stroke::new(20.0 / viewport.scale_factor(), egui::Color32::WHITE),
        );
        ui.painter().line(
            vec![
                Pos2::new(0.0, 0.0),
                Pos2::new(100.0, 100.0),
                Pos2::new(100.0, 500.0),
            ],
            egui::Stroke::new(2.0, egui::Color32::GOLD),
        );
        ui.painter()
            .circle_filled(egui::pos2(0.0, 0.0), 2.0, egui::Color32::RED);
        //workspace.board.layout().boundary()
    }
}
