// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use egui::Pos2;

use crate::{displayer::Displayer, workspace::Workspace};

pub struct Viewport {
    pub scene_rect: egui::Rect,
    pub ref_scene_rect: egui::Rect,
    pub scheduled_zoom_to_fit: bool,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            ref_scene_rect: egui::Rect::from_min_max(egui::pos2(-1.0, -1.0), egui::pos2(1.0, 1.0)),
            scheduled_zoom_to_fit: false,
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, workspace: Option<&mut Workspace>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut scene_rect = self.scene_rect.clone();

            egui::Scene::new()
                .zoom_range(0.0001..=10000.0)
                .show(ui, &mut scene_rect, |ui| {
                    if let Some(ref workspace) = workspace {
                        let mut displayer = Displayer::new();
                        displayer.update(ctx, ui, &self, workspace);
                    }
                });

            self.scene_rect = scene_rect;

            if let Some(workspace) = workspace {
                self.zoom_to_fit_if_scheduled(workspace);
            }
        });
    }

    fn zoom_to_fit_if_scheduled(&mut self, workspace: &Workspace) {
        if self.scheduled_zoom_to_fit {
            self.scene_rect = Self::boundary_bounding_box(workspace);
            self.ref_scene_rect = self.scene_rect.clone();
        }

        self.scheduled_zoom_to_fit = false;
    }

    fn boundary_bounding_box(workspace: &Workspace) -> egui::Rect {
        let first = workspace.board.layout().boundary()[0];

        let mut min_x = first[0];
        let mut max_x = first[0];
        let mut min_y = first[1];
        let mut max_y = first[1];

        for point in workspace.board.layout().boundary()[1..].iter() {
            if point[0] < min_x {
                min_x = point[0];
            }
            if point[0] > max_x {
                max_x = point[0];
            }
            if point[1] < min_y {
                min_y = point[1];
            }
            if point[1] > max_y {
                max_y = point[1];
            }
        }

        egui::Rect::from_min_max(
            Pos2::new(min_x as f32, min_y as f32),
            Pos2::new(max_x as f32, max_y as f32),
        )
        .scale_from_center(1.05)
    }

    pub fn scale_factor(&self) -> f32 {
        self.ref_scene_rect.width() / self.scene_rect.width()
    }
}
