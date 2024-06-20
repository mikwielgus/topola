use std::{
    fs::File,
    sync::{mpsc::Sender, Arc, Mutex},
};

use topola::{
    autorouter::invoker::{Command, Execute, Invoker, InvokerStatus},
    specctra::mesadata::SpecctraMesadata,
};

use crate::{
    app::{channel_text, execute, SharedData},
    overlay::Overlay,
};

pub struct Top {
    pub is_placing_via: bool,
    pub show_ratsnest: bool,
}

impl Top {
    pub fn new() -> Self {
        Self {
            is_placing_via: false,
            show_ratsnest: false,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        shared_data: Arc<Mutex<SharedData>>,
        sender: Sender<String>,
        maybe_invoker: &Option<Arc<Mutex<Invoker<SpecctraMesadata>>>>,
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
                        if let Some(invoker_arc_mutex) = &maybe_invoker {
                            let invoker_arc_mutex = invoker_arc_mutex.clone();
                            let ctx = ui.ctx().clone();
                            let task = rfd::AsyncFileDialog::new().pick_file();

                            execute(async move {
                                if let Some(file_handle) = task.await {
                                    let path = file_handle.path();
                                    let mut invoker = invoker_arc_mutex.lock().unwrap();
                                    let mut file = File::open(path).unwrap();
                                    invoker.replay(serde_json::from_reader(file).unwrap());
                                }
                            });
                        }
                    }

                    if ui.button("Save history").clicked() {
                        if let Some(invoker_arc_mutex) = &maybe_invoker {
                            let invoker_arc_mutex = invoker_arc_mutex.clone();
                            let ctx = ui.ctx().clone();
                            let task = rfd::AsyncFileDialog::new().save_file();

                            execute(async move {
                                if let Some(file_handle) = task.await {
                                    let path = file_handle.path();
                                    let mut invoker = invoker_arc_mutex.lock().unwrap();
                                    let mut file = File::create(path).unwrap();
                                    serde_json::to_writer_pretty(file, invoker.history());
                                }
                            });
                        }
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
                    if let (Some(invoker_arc_mutex), Some(overlay)) =
                        (&maybe_invoker, &maybe_overlay)
                    {
                        let invoker_arc_mutex = invoker_arc_mutex.clone();
                        let shared_data_arc_mutex = shared_data.clone();
                        let selection = overlay.selection().clone();

                        execute(async move {
                            let mut invoker = invoker_arc_mutex.lock().unwrap();
                            let mut execute = invoker.execute_walk(Command::Autoroute(selection));

                            if let Execute::Autoroute(ref mut autoroute) = execute {
                                let from = autoroute.navmesh().as_ref().unwrap().source();
                                let to = autoroute.navmesh().as_ref().unwrap().target();

                                {
                                    let mut shared_data = shared_data_arc_mutex.lock().unwrap();
                                    shared_data.from = Some(from);
                                    shared_data.to = Some(to);
                                    shared_data.navmesh = autoroute.navmesh().clone();
                                }
                            }

                            let _ = loop {
                                let status = match execute.step(&mut invoker) {
                                    Ok(status) => status,
                                    Err(err) => return,
                                };

                                if let InvokerStatus::Finished = status {
                                    break;
                                }

                                if let Execute::Autoroute(ref mut autoroute) = execute {
                                    shared_data_arc_mutex.lock().unwrap().navmesh =
                                        autoroute.navmesh().clone();
                                }
                            };
                        });
                    }
                }

                ui.toggle_value(&mut self.is_placing_via, "Place Via");

                ui.separator();

                if ui.button("Undo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z))
                {
                    if let Some(invoker_arc_mutex) = &maybe_invoker {
                        let invoker_arc_mutex = invoker_arc_mutex.clone();
                        execute(async move {
                            invoker_arc_mutex.lock().unwrap().undo();
                        });
                    }
                }

                if ui.button("Redo").clicked()
                    || ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y))
                {
                    if let Some(invoker_arc_mutex) = &maybe_invoker {
                        let invoker_arc_mutex = invoker_arc_mutex.clone();
                        execute(async move {
                            invoker_arc_mutex.lock().unwrap().redo();
                        });
                    }
                }

                ui.separator();

                ui.toggle_value(&mut self.show_ratsnest, "Show Ratsnest");

                ui.separator();

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
    }
}
