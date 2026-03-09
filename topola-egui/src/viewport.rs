// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{displayer::Displayer, workspace::Workspace};

pub struct Viewport {
    pub view_rect: egui::Rect,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            view_rect: egui::Rect::from_min_max(
                egui::pos2(-10000.0, 10000.0),
                egui::pos2(-10000.0, 10000.0),
            ),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, workspace: Option<&mut Workspace>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Scene::new()
                .zoom_range(0.0001..=10000.0)
                .show(ui, &mut self.view_rect, |ui| {
                    if let Some(workspace) = workspace {
                        let mut displayer = Displayer::new();
                        displayer.update(ctx, ui, workspace)
                    }
                });
        });
    }
}
