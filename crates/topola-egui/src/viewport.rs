// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::point;
use rstar::Envelope;
use topola::interactor::activity::{InteractiveEvent, InteractiveEventKind, InteractiveInput};

use crate::{
    config::Config, displayer::Displayer, error_dialog::ErrorDialog, menu_bar::MenuBar,
    painter::Painter, translator::Translator, workspace::Workspace,
};

pub struct Viewport {
    pub transform: egui::emath::TSTransform,
    /// how much should a single arrow key press scroll
    pub kbd_scroll_delta_factor: f32,
    pub scheduled_zoom_to_fit: bool,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            transform: egui::emath::TSTransform::new([0.0, 0.0].into(), 0.01),
            kbd_scroll_delta_factor: 5.0,
            scheduled_zoom_to_fit: false,
        }
    }

    pub fn update(
        &mut self,
        config: &Config,
        ctx: &egui::Context,
        tr: &Translator,
        menu_bar: &MenuBar,
        error_dialog: &mut ErrorDialog,
        maybe_workspace: Option<&mut Workspace>,
    ) -> egui::Rect {
        egui::CentralPanel::default()
            .show(ctx, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    // TODO: only request re-render if anything changed
                    ui.ctx().request_repaint();
                    let (id, viewport_rect) = ui.allocate_space(ui.available_size());

                    let (response, latest_pos) =
                        self.read_egui_response_and_latest_pos(id, viewport_rect, ctx, ui);
                    self.update_transform_by_input(ctx, latest_pos);

                    if let Some(workspace) = maybe_workspace {
                        let latest_point = point! {x: latest_pos.x as f64, y: -latest_pos.y as f64};

                        let interactive_input = InteractiveInput {
                            active_layer: Some(
                                menu_bar.multilayer_autoroute_options.planar.principal_layer,
                            ),
                            pointer_pos: latest_point,
                            dt: ctx.input(|i| {
                                if i.stable_dt <= i.predicted_dt {
                                    i.stable_dt
                                } else {
                                    i.predicted_dt
                                }
                            }),
                        };

                        workspace.advance_state_by_dt(
                            tr,
                            error_dialog,
                            menu_bar.frame_timestep,
                            &interactive_input,
                        );

                        let mut painter = Painter::new(ui, self.transform, menu_bar.show_bboxes);

                        let interactive_event_kind =
                            if response.clicked_by(egui::PointerButton::Primary) {
                                Some(InteractiveEventKind::PointerPrimaryButtonClicked)
                            } else if response.drag_started_by(egui::PointerButton::Primary) {
                                Some(InteractiveEventKind::PointerPrimaryButtonDragStarted)
                            } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                                Some(InteractiveEventKind::PointerPrimaryButtonDragStopped)
                            } else if response.clicked_by(egui::PointerButton::Secondary) {
                                Some(InteractiveEventKind::PointerSecondaryButtonClicked)
                            } else {
                                None
                            };
                        if let Some(kind) = interactive_event_kind {
                            let (ctrl, shift) = response
                                .ctx
                                .input(|i| (i.modifiers.ctrl, i.modifiers.shift));
                            let _ = workspace.update_state_for_event(
                                tr,
                                error_dialog,
                                menu_bar,
                                &interactive_input,
                                InteractiveEvent { kind, ctrl, shift },
                            );
                        } else if let Some((_, bsk, cur_bbox)) =
                            workspace.overlay.get_bbox_reselect(latest_point)
                        {
                            use topola::autorouter::selection::BboxSelectionKind;
                            painter.paint_bbox_with_color(
                                cur_bbox,
                                match bsk {
                                    BboxSelectionKind::CompletelyInside => egui::Color32::YELLOW,
                                    BboxSelectionKind::MerelyIntersects => egui::Color32::BLUE,
                                },
                            );
                        }

                        let mut displayer = Displayer::new(config, painter, workspace);
                        displayer.update(ctx, menu_bar);

                        self.zoom_to_fit_if_scheduled(&workspace, &viewport_rect);
                    }

                    viewport_rect
                })
            })
            .inner
            .inner
    }

    fn read_egui_response_and_latest_pos(
        &self,
        id: egui::Id,
        viewport_rect: egui::Rect,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
    ) -> (egui::Response, egui::Pos2) {
        let response = ui.interact(viewport_rect, id, egui::Sense::click_and_drag());

        // NOTE: we use `interact_pos` instead of `latest_pos` to handle "pointer gone"
        // events more graceful
        let latest_pos = self.transform.inverse()
            * (response
                .interact_pointer_pos()
                .unwrap_or_else(|| ctx.input(|i| i.pointer.interact_pos().unwrap_or_default())));

        // disable built-in behavior of arrow keys
        if response.has_focus() {
            response.ctx.memory_mut(|m| {
                // we are only allowed to modify the focus lock filter if we have focus
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
        }

        (response, latest_pos)
    }

    fn update_transform_by_input(&mut self, ctx: &egui::Context, latest_pos: egui::Pos2) {
        let old_scaling = self.transform.scaling;
        self.transform.scaling *= ctx.input(|i| i.zoom_delta());

        self.transform.translation += latest_pos.to_vec2() * (old_scaling - self.transform.scaling);
        self.transform.translation += ctx.input_mut(|i| {
            // handle scrolling
            let mut scroll_delta = core::mem::take(&mut i.smooth_scroll_delta);

            // arrow keys
            let kbd_sdf = self.kbd_scroll_delta_factor;
            let mut pressed =
                |key| i.consume_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, key));
            use egui::Key;
            scroll_delta.y += if pressed(Key::ArrowDown) {
                kbd_sdf
            } else if pressed(Key::ArrowUp) {
                -kbd_sdf
            } else {
                0.0
            };
            scroll_delta.x += if pressed(Key::ArrowRight) {
                kbd_sdf
            } else if pressed(Key::ArrowLeft) {
                -kbd_sdf
            } else {
                0.0
            };

            scroll_delta
        });
    }

    fn zoom_to_fit_if_scheduled(&mut self, workspace: &Workspace, viewport_rect: &egui::Rect) {
        if self.scheduled_zoom_to_fit {
            let root_bbox = workspace
                .interactor
                .invoker()
                .autorouter()
                .board()
                .layout()
                .drawing()
                .rtree()
                .root()
                .envelope();

            let root_bbox_width = root_bbox.upper()[0] - root_bbox.lower()[0];
            let root_bbox_height = root_bbox.upper()[1] - root_bbox.lower()[1];

            self.transform.scaling = 0.8
                * if root_bbox_width / root_bbox_height
                    >= (viewport_rect.width() as f64) / (viewport_rect.height() as f64)
                {
                    viewport_rect.width() / root_bbox_width as f32
                } else {
                    viewport_rect.height() / root_bbox_height as f32
                };

            self.transform.translation =
                egui::Vec2::new(viewport_rect.center()[0], viewport_rect.center()[1])
                    - (self.transform.scaling
                        * egui::Pos2::new(
                            root_bbox.center()[0] as f32,
                            -root_bbox.center()[1] as f32,
                        ))
                    .to_vec2();
        }

        self.scheduled_zoom_to_fit = false;
    }
}
