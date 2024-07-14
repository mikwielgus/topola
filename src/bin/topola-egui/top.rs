use std::{
    fs::File,
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};

use topola::{
    autorouter::invoker::{
        Command, Execute, ExecuteWithStatus, Invoker, InvokerError, InvokerStatus,
    },
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

use crate::{
    action::{Action, Switch, Trigger},
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
        maybe_overlay: &mut Option<Overlay>,
        maybe_design: &Option<SpecctraDesign>,
    ) -> Result<(), InvokerError> {
        let mut open_design =
            Trigger::new(Action::new("Open", egui::Modifiers::CTRL, egui::Key::O));
        let mut export_session = Trigger::new(Action::new(
            "Export session file",
            egui::Modifiers::CTRL,
            egui::Key::S,
        ));
        let mut import_history = Trigger::new(Action::new(
            "Import history",
            egui::Modifiers::CTRL,
            egui::Key::I,
        ));
        let mut export_history = Trigger::new(Action::new(
            "Export history",
            egui::Modifiers::CTRL,
            egui::Key::E,
        ));
        let mut quit = Trigger::new(Action::new("Quit", egui::Modifiers::CTRL, egui::Key::V));
        let mut autoroute = Trigger::new(Action::new(
            "Autoroute",
            egui::Modifiers::CTRL,
            egui::Key::A,
        ));
        let mut place_via = Switch::new(Action::new(
            "Place Via",
            egui::Modifiers::CTRL,
            egui::Key::P,
        ));
        let mut undo = Trigger::new(Action::new("Undo", egui::Modifiers::CTRL, egui::Key::Z));
        let mut redo = Trigger::new(Action::new("Redo", egui::Modifiers::CTRL, egui::Key::Y));

        egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        open_design.button(ctx, ui);
                        export_session.button(ctx, ui);

                        ui.separator();

                        import_history.button(ctx, ui);
                        export_history.button(ctx, ui);

                        ui.separator();

                        // "Quit" button wouldn't work on a Web page.
                        if !cfg!(target_arch = "wasm32") {
                            quit.button(ctx, ui);
                        }
                    });

                    ui.separator();

                    autoroute.button(ctx, ui);

                    place_via.toggle_widget(ctx, ui, &mut self.is_placing_via);

                    ui.separator();

                    undo.button(ctx, ui);
                    redo.button(ctx, ui);

                    ui.separator();

                    ui.toggle_value(&mut self.show_ratsnest, "Show Ratsnest");
                    ui.toggle_value(&mut self.show_navmesh, "Show Navmesh");

                    ui.separator();

                    egui::widgets::global_dark_light_mode_buttons(ui);
                });

                if open_design.consume_key_triggered(ctx, ui) {
                    // NOTE: On Linux, this requires Zenity to be installed on your system.
                    let ctx = ctx.clone();
                    let task = rfd::AsyncFileDialog::new().pick_file();

                    execute(async move {
                        if let Some(file_handle) = task.await {
                            let file_sender = FileSender::new(content_sender);
                            file_sender.send(file_handle).await;
                            ctx.request_repaint();
                        }
                    });
                } else if export_session.consume_key_triggered(ctx, ui) {
                    if let Some(design) = maybe_design {
                        if let Some(invoker) =
                            arc_mutex_maybe_invoker.clone().lock().unwrap().as_ref()
                        {
                            let ctx = ui.ctx().clone();
                            let board = invoker.autorouter().board();

                            // FIXME: I don't know how to avoid buffering the entire exported file
                            let mut writebuf = vec![];

                            design.write_ses(board, &mut writebuf);

                            let mut dialog = rfd::AsyncFileDialog::new();
                            if let Some(filename) = Path::new(design.get_name()).file_stem() {
                                if let Some(filename) = filename.to_str() {
                                    dialog = dialog.set_file_name(filename);
                                }
                            }
                            let task = dialog
                                .add_filter("Specctra session file", &["ses"])
                                .save_file();

                            execute(async move {
                                if let Some(file_handle) = task.await {
                                    file_handle.write(&writebuf).await;
                                    ctx.request_repaint();
                                }
                            });
                        }
                    }
                } else if import_history.consume_key_triggered(ctx, ui) {
                    let ctx = ctx.clone();
                    let task = rfd::AsyncFileDialog::new().pick_file();

                    execute(async move {
                        if let Some(file_handle) = task.await {
                            let file_sender = FileSender::new(history_sender);
                            file_sender.send(file_handle).await;
                            ctx.request_repaint();
                        }
                    });
                } else if export_history.consume_key_triggered(ctx, ui) {
                    if let Some(invoker) = arc_mutex_maybe_invoker.clone().lock().unwrap().as_ref()
                    {
                        let ctx = ctx.clone();
                        let task = rfd::AsyncFileDialog::new().save_file();

                        // FIXME: I don't think we should be buffering everything in a `Vec<u8>`.
                        let mut writebuf = vec![];
                        serde_json::to_writer_pretty(&mut writebuf, invoker.history());

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                file_handle.write(&writebuf).await;
                                ctx.request_repaint();
                            }
                        });
                    }
                } else if quit.consume_key_triggered(ctx, ui) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else if autoroute.consume_key_triggered(ctx, ui) {
                    if maybe_execute.as_mut().map_or(true, |execute| {
                        matches!(execute.maybe_status(), Some(InvokerStatus::Finished))
                    }) {
                        if let (Some(invoker), Some(ref mut overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.selection().clone();
                            overlay.clear_selection();
                            maybe_execute.insert(ExecuteWithStatus::new(
                                invoker.execute_walk(Command::Autoroute(selection))?,
                            ));
                        }
                    }
                } else if place_via.consume_key_enabled(ctx, ui, &mut self.is_placing_via) {
                } else if undo.consume_key_triggered(ctx, ui) {
                    if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
                        invoker.undo();
                    }
                } else if redo.consume_key_triggered(ctx, ui) {
                    if let Some(ref mut invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut()
                    {
                        invoker.redo();
                    }
                }

                Ok::<(), InvokerError>(())
            })
            .inner
    }
}
