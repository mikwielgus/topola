use std::{
    fs::File,
    sync::{mpsc::Sender, Arc, Mutex},
};

use topola::{
    autorouter::invoker::{Command, Execute, ExecuteWithStatus, Invoker, InvokerStatus},
    specctra::mesadata::SpecctraMesadata,
};

use crate::{
    app::{channel_text, execute},
    file_sender::FileSender,
    overlay::Overlay,
};

pub struct Top {
    pub is_placing_via: bool,
    pub show_ratsnest: bool,
    pub show_navmesh: bool,
}

impl Top {
    pub fn new() -> Self {
        Self {
            is_placing_via: false,
            show_ratsnest: false,
            show_navmesh: false,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        content_sender: Sender<String>,
        history_sender: Sender<String>,
        arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,
        maybe_execute: &mut Option<ExecuteWithStatus>,
        maybe_overlay: &Option<Overlay>,
    ) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        // `Context` is cheap to clone as it's wrapped in an `Arc`.
                        let ctx = ui.ctx().clone();
                        // NOTE: On Linux, this requires Zenity to be installed on your system.
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                let file_sender = FileSender::new(content_sender);
                                file_sender.send(file_handle).await;
                                ctx.request_repaint();
                            }
                        });
                    }

                    ui.separator();

                    if ui.button("Load history").clicked() {
                        let invoker_arc_mutex = arc_mutex_maybe_invoker.clone();
                        let ctx = ui.ctx().clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                let file_sender = FileSender::new(history_sender);
                                file_sender.send(file_handle).await;
                                ctx.request_repaint();
                            }
                        });
                    } else if ui.button("Save history").clicked() {
                        let invoker_arc_mutex = arc_mutex_maybe_invoker.clone();
                        let ctx = ui.ctx().clone();
                        let task = rfd::AsyncFileDialog::new().save_file();

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                let file_sender = FileSender::new(history_sender);
                                file_sender.send(file_handle).await;
                                ctx.request_repaint();
                            }
                        });
                    }

                    ui.separator();

                    // "Quit" button wouldn't work on a Web page.
                    if !cfg!(target_arch = "wasm32") {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                });

                ui.separator();

                if ui.button("Autoroute").clicked() {
                    if maybe_execute.as_mut().map_or(true, |execute| {
                        matches!(execute.maybe_status(), Some(InvokerStatus::Finished))
                    }) {
                        if let (Some(invoker), Some(ref overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.selection().clone();
                            maybe_execute.insert(ExecuteWithStatus::new(
                                invoker.execute_walk(Command::Autoroute(selection)),
                            ));
                        }
                    }
                }

                ui.toggle_value(&mut self.is_placing_via, "Place Via");

                ui.separator();

                if ui.button("Undo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z))
                {
                    if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
                        invoker.undo();
                    }
                }

                if ui.button("Redo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y))
                {
                    if let Some(ref mut invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut()
                    {
                        invoker.redo();
                    }
                }

                ui.separator();

                ui.toggle_value(&mut self.show_ratsnest, "Show Ratsnest");
                ui.toggle_value(&mut self.show_navmesh, "Show Navmesh");

                ui.separator();

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
    }
}
