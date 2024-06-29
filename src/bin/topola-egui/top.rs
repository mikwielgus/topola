use std::{
    fs::File,
    sync::{mpsc::Sender, Arc, Mutex},
};

use topola::{
    autorouter::invoker::{Command, Execute, Invoker, InvokerStatus},
    specctra::mesadata::SpecctraMesadata,
};

use crate::{
    app::{channel_text, execute},
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
        sender: Sender<String>,
        maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,
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
                                sender.send(channel_text(file_handle).await);
                                ctx.request_repaint();
                            }
                        });
                    }

                    ui.separator();

                    if ui.button("Load history").clicked() {
                        let invoker_arc_mutex = maybe_invoker.clone();
                        let ctx = ui.ctx().clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();

                        execute(async move {
                            let Some(file_handle) = task.await else {
                                return;
                            };

                            let path = file_handle.path();
                            let Ok(mut file) = File::open(path) else {
                                return;
                            };

                            let mut locked_invoker = invoker_arc_mutex.lock().unwrap();
                            let Some(mut invoker) = locked_invoker.as_mut() else {
                                return;
                            };

                            invoker.replay(serde_json::from_reader(file).unwrap());
                        });
                    }

                    if ui.button("Save history").clicked() {
                        let invoker_arc_mutex = maybe_invoker.clone();
                        let ctx = ui.ctx().clone();
                        let task = rfd::AsyncFileDialog::new().save_file();

                        execute(async move {
                            let Some(file_handle) = task.await else {
                                return;
                            };

                            let path = file_handle.path();
                            let Ok(mut file) = File::create(path) else {
                                return;
                            };

                            let mut locked_invoker = invoker_arc_mutex.lock().unwrap();
                            let Some(mut invoker) = locked_invoker.as_mut() else {
                                return;
                            };

                            serde_json::to_writer_pretty(file, invoker.history());
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
                    if let (Some(invoker), Some(ref overlay)) =
                        (maybe_invoker.lock().unwrap().as_mut(), maybe_overlay)
                    {
                        let selection = overlay.selection().clone();
                        let mut execute = invoker.execute_walk(Command::Autoroute(selection));

                        if let Execute::Autoroute(ref mut autoroute) = execute {
                            let from = autoroute.navmesh().as_ref().unwrap().source();
                            let to = autoroute.navmesh().as_ref().unwrap().target();

                            /*{
                                let mut shared_data = shared_data_arc_mutex.lock().unwrap();
                                shared_data.from = Some(from);
                                shared_data.to = Some(to);
                                shared_data.navmesh = autoroute.navmesh().cloned();
                            }*/
                        }

                        let _ = loop {
                            let status = match execute.step(invoker) {
                                Ok(status) => status,
                                Err(err) => return,
                            };

                            if let InvokerStatus::Finished = status {
                                break;
                            }

                            /*if let Execute::Autoroute(ref mut autoroute) = execute {
                                shared_data_arc_mutex.lock().unwrap().navmesh =
                                    autoroute.navmesh().cloned();
                            }*/
                        };
                    }
                }

                ui.toggle_value(&mut self.is_placing_via, "Place Via");

                ui.separator();

                if ui.button("Undo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z))
                {
                    if let Some(invoker) = maybe_invoker.lock().unwrap().as_mut() {
                        invoker.undo();
                    }
                }

                if ui.button("Redo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y))
                {
                    if let Some(ref mut invoker) = maybe_invoker.lock().unwrap().as_mut() {
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
