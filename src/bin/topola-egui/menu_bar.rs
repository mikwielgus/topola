use std::{
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};

use topola::{
    autorouter::{
        command::Command,
        history::History,
        invoker::{Invoker, InvokerError},
        AutorouterOptions,
    },
    router::RouterOptions,
    specctra::{
        design::{LoadingError as SpecctraLoadingError, SpecctraDesign},
        mesadata::SpecctraMesadata,
    },
    stepper::Abort,
};

use crate::{
    action::{Action, Switch, Trigger},
    activity::{ActivityStatus, ActivityStepperWithStatus},
    app::{execute, handle_file},
    overlay::Overlay,
    translator::Translator,
    viewport::Viewport,
};

pub struct MenuBar {
    pub autorouter_options: AutorouterOptions,
    pub is_placing_via: bool,
    pub show_ratsnest: bool,
    pub show_navmesh: bool,
    pub show_bboxes: bool,
    pub show_origin_destination: bool,
    pub show_layer_manager: bool,
    pub frame_timestep: f32,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            autorouter_options: AutorouterOptions {
                presort_by_pairwise_detours: false,
                router_options: RouterOptions {
                    wrap_around_bands: true,
                    squeeze_through_under_bands: true,
                },
            },
            is_placing_via: false,
            show_ratsnest: false,
            show_navmesh: false,
            show_bboxes: false,
            show_origin_destination: false,
            show_layer_manager: true,
            frame_timestep: 0.1,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        tr: &Translator,
        content_sender: Sender<Result<SpecctraDesign, SpecctraLoadingError>>,
        history_sender: Sender<std::io::Result<Result<History, serde_json::Error>>>,
        arc_mutex_maybe_invoker: Arc<Mutex<Option<Invoker<SpecctraMesadata>>>>,
        maybe_activity: &mut Option<ActivityStepperWithStatus>,
        viewport: &mut Viewport,
        maybe_overlay: &mut Option<Overlay>,
        maybe_design: &Option<SpecctraDesign>,
    ) -> Result<(), InvokerError> {
        let mut open_design = Trigger::new(Action::new(
            tr.text("tr-menu-file-open"),
            egui::Modifiers::CTRL,
            egui::Key::O,
        ));
        let mut export_session = Trigger::new(Action::new(
            tr.text("tr-menu-file-export-session-file"),
            egui::Modifiers::CTRL,
            egui::Key::S,
        ));
        let mut import_history = Trigger::new(Action::new(
            tr.text("tr-menu-file-import-history"),
            egui::Modifiers::CTRL,
            egui::Key::I,
        ));
        let mut export_history = Trigger::new(Action::new(
            tr.text("tr-menu-file-export-history"),
            egui::Modifiers::CTRL,
            egui::Key::E,
        ));
        let mut quit = Trigger::new(Action::new(
            tr.text("tr-menu-file-quit"),
            egui::Modifiers::CTRL,
            egui::Key::Q,
        ));
        let mut undo = Trigger::new(Action::new(
            tr.text("tr-menu-edit_undo"),
            egui::Modifiers::CTRL,
            egui::Key::Z,
        ));
        let mut redo = Trigger::new(Action::new(
            tr.text("tr-menu-edit_redo"),
            egui::Modifiers::CTRL,
            egui::Key::Y,
        ));
        let mut abort = Trigger::new(Action::new(
            tr.text("tr-menu-edit_abort"),
            egui::Modifiers::NONE,
            egui::Key::Escape,
        ));
        let mut remove_bands = Trigger::new(Action::new(
            tr.text("tr-menu-edit_remove-bands"),
            egui::Modifiers::NONE,
            egui::Key::Delete,
        ));
        let mut place_via = Switch::new(Action::new(
            tr.text("tr-menu-place-place-via"),
            egui::Modifiers::CTRL,
            egui::Key::P,
        ));
        let mut autoroute = Trigger::new(Action::new(
            tr.text("tr-menu-route-autoroute"),
            egui::Modifiers::CTRL,
            egui::Key::A,
        ));
        let mut compare_detours = Trigger::new(Action::new(
            tr.text("tr-menu-inspect_compare-detours"),
            egui::Modifiers::NONE,
            egui::Key::Minus,
        ));
        let mut measure_length = Trigger::new(Action::new(
            tr.text("tr-menu-inspect_measure-length"),
            egui::Modifiers::NONE,
            egui::Key::Plus,
        ));

        egui::TopBottomPanel::top("menu_bar")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(tr.text("tr-menu-file"), |ui| {
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

                    ui.menu_button(tr.text("tr-menu-edit"), |ui| {
                        undo.button(ctx, ui);
                        redo.button(ctx, ui);

                        ui.separator();

                        abort.button(ctx, ui);

                        ui.separator();

                        remove_bands.button(ctx, ui);
                    });

                    ui.menu_button(tr.text("tr-menu-view"), |ui| {
                        ui.toggle_value(
                            &mut viewport.scheduled_zoom_to_fit,
                            tr.text("tr-menu-view_zoom-to-fit"),
                        );

                        ui.separator();

                        ui.checkbox(&mut self.show_ratsnest, tr.text("tr-menu-view_show-ratsnest"));
                        ui.checkbox(&mut self.show_navmesh, tr.text("tr-menu-view_show-navmesh"));
                        ui.checkbox(&mut self.show_bboxes, tr.text("tr-menu-view_show-bboxes"));
                        ui.checkbox(
                            &mut self.show_origin_destination,
                            tr.text("tr-menu-view_show-origin-destination"),
                        );

                        ui.separator();

                        ui.checkbox(&mut self.show_layer_manager, tr.text("tr-menu-view_show-layer-manager"));

                        ui.separator();

                        ui.label(tr.text("tr-menu-view_frame-timestep"));
                        ui.add(
                            egui::widgets::Slider::new(&mut self.frame_timestep, 0.0..=3.0)
                                .suffix(" s"),
                        );
                    });

                    ui.menu_button(tr.text("tr-menu-place"), |ui| {
                        place_via.toggle_widget(ctx, ui, &mut self.is_placing_via);
                    });

                    ui.menu_button(tr.text("tr-menu-route"), |ui| {
                        autoroute.button(ctx, ui);
                        ui.separator();

                        ui.menu_button(tr.text("tr-menu-options"), |ui| {
                            ui.checkbox(
                                &mut self.autorouter_options.presort_by_pairwise_detours,
                                tr.text("tr-menu-route-options_presort-by-pairwise-detours"),
                            );
                            ui.checkbox(
                                &mut self
                                    .autorouter_options
                                    .router_options
                                    .squeeze_through_under_bands,
                                tr.text("tr-menu-route-options_squeeze-through-under-bands"),
                            );
                            ui.checkbox(
                                &mut self.autorouter_options.router_options.wrap_around_bands,
                                tr.text("tr-menu-route-options_wrap-around-bands"),
                            );
                        });
                    });

                    ui.menu_button(tr.text("tr-menu-inspect"), |ui| {
                        compare_detours.button(ctx, ui);
                        measure_length.button(ctx, ui);
                    });

                    ui.separator();

                    egui::widgets::global_dark_light_mode_buttons(ui);
                });

                if open_design.consume_key_triggered(ctx, ui) {
                    // NOTE: On Linux, this requires Zenity to be installed on your system.
                    let ctx = ctx.clone();
                    let task = rfd::AsyncFileDialog::new().pick_file();

                    execute(async move {
                        if let Some(file_handle) = task.await {
                            let data = handle_file(&file_handle)
                                .await
                                .map_err(Into::into)
                                .and_then(SpecctraDesign::load);
                            content_sender.send(data);
                            ctx.request_repaint();
                        }
                    });
                } else if export_session.consume_key_triggered(ctx, ui) {
                    if let Some(design) = maybe_design {
                        if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_ref() {
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
                                .add_filter(tr.text("tr-menu-open_specctra-session-file"), &["ses"])
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
                            let data = handle_file(&file_handle).await.and_then(|data| {
                                match serde_json::from_reader(data) {
                                    Ok(history) => Ok(Ok(history)),
                                    Err(err) if err.is_io() => Err(err.into()),
                                    Err(err) => Ok(Err(err)),
                                }
                            });
                            history_sender.send(data);
                            ctx.request_repaint();
                        }
                    });
                } else if export_history.consume_key_triggered(ctx, ui) {
                    if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_ref() {
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
                } else if undo.consume_key_triggered(ctx, ui) {
                    if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
                        invoker.undo();
                    }
                } else if redo.consume_key_triggered(ctx, ui) {
                    if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
                        invoker.redo();
                    }
                } else if abort.consume_key_triggered(ctx, ui) {
                    if let Some(activity) = maybe_activity {
                        if let Some(invoker) = arc_mutex_maybe_invoker.lock().unwrap().as_mut() {
                            activity.abort(invoker);
                        }
                    }
                } else if remove_bands.consume_key_triggered(ctx, ui) {
                    if maybe_activity.as_mut().map_or(true, |activity| {
                        matches!(activity.maybe_status(), Some(ActivityStatus::Finished(..)))
                    }) {
                        if let (Some(invoker), Some(ref mut overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.take_selection();
                            *maybe_activity = Some(ActivityStepperWithStatus::new_execution(
                                invoker.execute_stepper(Command::RemoveBands(
                                    selection.band_selection,
                                ))?,
                            ));
                        }
                    }
                } else if place_via.consume_key_enabled(ctx, ui, &mut self.is_placing_via) {
                } else if autoroute.consume_key_triggered(ctx, ui) {
                    if maybe_activity.as_mut().map_or(true, |activity| {
                        matches!(activity.maybe_status(), Some(ActivityStatus::Finished(..)))
                    }) {
                        if let (Some(invoker), Some(ref mut overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.take_selection();
                            *maybe_activity = Some(ActivityStepperWithStatus::new_execution(
                                invoker.execute_stepper(Command::Autoroute(
                                    selection.pin_selection,
                                    self.autorouter_options,
                                ))?,
                            ));
                        }
                    }
                } else if compare_detours.consume_key_triggered(ctx, ui) {
                    if maybe_activity.as_mut().map_or(true, |activity| {
                        matches!(activity.maybe_status(), Some(ActivityStatus::Finished(..)))
                    }) {
                        if let (Some(invoker), Some(ref mut overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.take_selection();
                            *maybe_activity = Some(ActivityStepperWithStatus::new_execution(
                                invoker.execute_stepper(Command::CompareDetours(
                                    selection.pin_selection,
                                    self.autorouter_options,
                                ))?,
                            ));
                        }
                    }
                } else if measure_length.consume_key_triggered(ctx, ui) {
                    if maybe_activity.as_mut().map_or(true, |activity| {
                        matches!(activity.maybe_status(), Some(ActivityStatus::Finished(..)))
                    }) {
                        if let (Some(invoker), Some(ref mut overlay)) = (
                            arc_mutex_maybe_invoker.lock().unwrap().as_mut(),
                            maybe_overlay,
                        ) {
                            let selection = overlay.take_selection();
                            *maybe_activity = Some(ActivityStepperWithStatus::new_execution(
                                invoker.execute_stepper(Command::MeasureLength(
                                    selection.band_selection,
                                ))?,
                            ));
                        }
                    }
                }
                Ok::<(), InvokerError>(())
            })
            .inner
    }
}
