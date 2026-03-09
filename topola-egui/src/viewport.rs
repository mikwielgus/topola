// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{displayer::Displayer, workspace::Workspace};

pub struct Viewport {
    pub scene_rect: egui::Rect,
    pub ref_scene_rect: egui::Rect,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_max(
                egui::pos2(-10000.0, -10000.0),
                egui::pos2(10000.0, 10000.0),
            ),
            ref_scene_rect: egui::Rect::from_min_max(
                egui::pos2(-10000.0, -10000.0),
                egui::pos2(10000.0, 10000.0),
            ),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, workspace: Option<&mut Workspace>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut scene_rect = self.scene_rect.clone();

            egui::Scene::new()
                .zoom_range(0.0001..=10000.0)
                .show(ui, &mut scene_rect, |ui| {
                    if let Some(workspace) = workspace {
                        let mut displayer = Displayer::new();
                        displayer.update(ctx, ui, &self, workspace);
                    }
                });

            self.scene_rect = scene_rect;
        });
    }

    pub fn scale_factor(&self) -> f32 {
        self.ref_scene_rect.width() / self.scene_rect.width()
    }
}
