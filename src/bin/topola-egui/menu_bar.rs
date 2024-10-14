use std::{ops::ControlFlow, path::Path, sync::mpsc::Sender};

use topola::{
    autorouter::{
        execution::Command, invoker::InvokerError, selection::Selection, AutorouterOptions,
    },
    interactor::{
        activity::{ActivityContext, ActivityStepperWithStatus},
        interaction::InteractionContext,
    },
    router::RouterOptions,
    specctra::design::{LoadingError as SpecctraLoadingError, SpecctraDesign},
    stepper::Abort,
};

use crate::{
    action::{Action, Switch, Trigger},
    actions::Actions,
    app::{execute, handle_file},
    translator::Translator,
    viewport::Viewport,
    workspace::Workspace,
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
        tr: &mut Translator,
        content_sender: Sender<Result<SpecctraDesign, SpecctraLoadingError>>,
        viewport: &mut Viewport,
        maybe_workspace: Option<&mut Workspace>,
    ) -> Result<(), InvokerError> {
        let mut actions = Actions::new(tr);
        let online_documentation_url = "https://topola.codeberg.page/doc/";

        let workspace_activities_enabled = match &maybe_workspace {
            Some(w) => w
                .interactor
                .maybe_activity()
                .as_ref()
                .map_or(true, |activity| {
                    matches!(activity.maybe_status(), Some(ControlFlow::Break(..)))
                }),
            None => false,
        };

        egui::TopBottomPanel::top("menu_bar")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(tr.text("tr-menu-file"), |ui| {
                        actions.file.open_design.button(ctx, ui);
                        //ui.add_enabled_ui(maybe_workspace.is_some(), |ui| {
                        actions.file.export_session.button(ctx, ui);

                        ui.separator();

                        actions.file.import_history.button(ctx, ui);
                        actions.file.export_history.button(ctx, ui);
                        //});

                        ui.separator();

                        // "Quit" button wouldn't work on a Web page.
                        if !cfg!(target_arch = "wasm32") {
                            actions.file.quit.button(ctx, ui);
                        }
                    });

                    ui.menu_button(tr.text("tr-menu-edit"), |ui| {
                        ui.add_enabled_ui(maybe_workspace.is_some(), |ui| {
                            actions.edit.undo.button(ctx, ui);
                            actions.edit.redo.button(ctx, ui);

                            ui.separator();

                            actions.edit.abort.button(ctx, ui);

                            ui.separator();

                            //ui.add_enabled_ui(workspace_activities_enabled, |ui| {
                            actions.edit.remove_bands.button(ctx, ui);
                            //});
                        });
                    });

                    self.update_view_menu(ctx, ui, tr, viewport);

                    // NOTE: we could disable the entire range of menus below
                    // when no workspace is loaded, but that would disrupt "hover-scrolling"
                    // between menus inside of the conditionally enabled section and
                    // those outside...

                    ui.menu_button(tr.text("tr-menu-place"), |ui| {
                        ui.add_enabled_ui(maybe_workspace.is_some(), |ui| {
                            actions.place.place_via.toggle_widget(
                                ctx,
                                ui,
                                &mut self.is_placing_via,
                            );
                        });
                    });

                    ui.menu_button(tr.text("tr-menu-route"), |ui| {
                        ui.add_enabled_ui(maybe_workspace.is_some(), |ui| {
                            //ui.add_enabled_ui(workspace_activities_enabled, |ui| {
                            actions.route.autoroute.button(ctx, ui);
                            //});
                            ui.separator();

                            ui.menu_button(tr.text("tr-menu-options"), |ui| {
                                ui.checkbox(
                                    &mut self.autorouter_options.presort_by_pairwise_detours,
                                    tr.text("tr-menu-route-options-presort-by-pairwise-detours"),
                                );
                                ui.checkbox(
                                    &mut self
                                        .autorouter_options
                                        .router_options
                                        .squeeze_through_under_bands,
                                    tr.text("tr-menu-route-options-squeeze-through-under-bands"),
                                );
                                ui.checkbox(
                                    &mut self.autorouter_options.router_options.wrap_around_bands,
                                    tr.text("tr-menu-route-options-wrap-around-bands"),
                                );
                            });
                        });
                    });

                    ui.menu_button(tr.text("tr-menu-inspect"), |ui| {
                        ui.add_enabled_ui(workspace_activities_enabled, |ui| {
                            actions.inspect.compare_detours.button(ctx, ui);
                            actions.inspect.measure_length.button(ctx, ui);
                        });
                    });

                    ui.menu_button(tr.text("tr-menu-properties"), |ui| {
                        ui.menu_button(tr.text("tr-menu-properties-set-language"), |ui| {
                            for langid in Translator::locales() {
                                ui.radio_value(
                                    tr.langid_mut(),
                                    langid.clone(),
                                    langid.language.as_str(),
                                );
                                //ui.add(egui::RadioButton::new(true, locale.language.as_str()));
                            }
                        });
                    });

                    ui.menu_button(tr.text("tr-menu-help"), |ui| {
                        actions.help.online_documentation.hyperlink(
                            ctx,
                            ui,
                            online_documentation_url,
                        );
                    });

                    ui.separator();

                    egui::widgets::global_theme_preference_buttons(ui);
                });

                if actions.file.open_design.consume_key_triggered(ctx, ui) {
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
                } else if actions.file.quit.consume_key_triggered(ctx, ui) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else if actions
                    .help
                    .online_documentation
                    .consume_key_triggered(ctx, ui)
                {
                    ui.ctx().open_url(egui::OpenUrl {
                        url: String::from(online_documentation_url),
                        new_tab: true,
                    });
                } else if let Some(workspace) = maybe_workspace {
                    if actions.file.export_session.consume_key_triggered(ctx, ui) {
                        let ctx = ui.ctx().clone();
                        let board = workspace.interactor.invoker().autorouter().board();

                        // FIXME: I don't know how to avoid buffering the entire exported file
                        let mut writebuf = vec![];

                        workspace.design.write_ses(board, &mut writebuf);

                        let mut dialog = rfd::AsyncFileDialog::new();
                        if let Some(filename) = Path::new(workspace.design.get_name()).file_stem() {
                            if let Some(filename) = filename.to_str() {
                                dialog = dialog.set_file_name(filename);
                            }
                        }

                        let task = dialog
                            .add_filter(tr.text("tr-menu-open-specctra-session-file"), &["ses"])
                            .save_file();

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                file_handle.write(&writebuf).await;
                                ctx.request_repaint();
                            }
                        });
                    } else if actions.file.import_history.consume_key_triggered(ctx, ui) {
                        let ctx = ctx.clone();
                        let task = rfd::AsyncFileDialog::new().pick_file();
                        let history_sender = workspace.history_channel.0.clone();

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
                    } else if actions.file.export_history.consume_key_triggered(ctx, ui) {
                        let ctx = ctx.clone();
                        let task = rfd::AsyncFileDialog::new().save_file();

                        // FIXME: I don't think we should be buffering everything in a `Vec<u8>`.
                        let mut writebuf = vec![];
                        serde_json::to_writer_pretty(
                            &mut writebuf,
                            workspace.interactor.invoker().history(),
                        );

                        execute(async move {
                            if let Some(file_handle) = task.await {
                                file_handle.write(&writebuf).await;
                                ctx.request_repaint();
                            }
                        });
                    } else if actions.edit.undo.consume_key_triggered(ctx, ui) {
                        workspace.interactor.undo();
                    } else if actions.edit.redo.consume_key_triggered(ctx, ui) {
                        workspace.interactor.redo();
                    } else if actions.edit.abort.consume_key_triggered(ctx, ui) {
                        workspace.interactor.abort()
                    } else if actions.place.place_via.consume_key_enabled(
                        ctx,
                        ui,
                        &mut self.is_placing_via,
                    ) {
                    } else if workspace_activities_enabled {
                        let mut schedule = |op: fn(Selection, AutorouterOptions) -> Command| {
                            let selection = workspace.overlay.take_selection();
                            workspace
                                .interactor
                                .schedule(op(selection, self.autorouter_options));
                            Ok::<(), InvokerError>(())
                        };
                        if actions.edit.remove_bands.consume_key_triggered(ctx, ui) {
                            schedule(|selection, _| {
                                Command::RemoveBands(selection.band_selection)
                            })?;
                        } else if actions.route.autoroute.consume_key_triggered(ctx, ui) {
                            schedule(|selection, opts| {
                                Command::Autoroute(selection.pin_selection, opts)
                            })?;
                        } else if actions
                            .inspect
                            .compare_detours
                            .consume_key_triggered(ctx, ui)
                        {
                            schedule(|selection, opts| {
                                Command::CompareDetours(selection.pin_selection, opts)
                            })?;
                        } else if actions
                            .inspect
                            .measure_length
                            .consume_key_triggered(ctx, ui)
                        {
                            schedule(|selection, _| {
                                Command::MeasureLength(selection.band_selection)
                            })?;
                        }
                    }
                }
                Ok::<(), InvokerError>(())
            })
            .inner
    }

    pub fn update_view_menu(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        tr: &Translator,
        viewport: &mut Viewport,
    ) {
        ui.menu_button(tr.text("tr-menu-view"), |ui| {
            ui.toggle_value(
                &mut viewport.scheduled_zoom_to_fit,
                tr.text("tr-menu-view-zoom-to-fit"),
            );

            ui.separator();

            //ui.add_enabled_ui(maybe_workspace.is_some(), |ui| {
            ui.checkbox(
                &mut self.show_ratsnest,
                tr.text("tr-menu-view-show-ratsnest"),
            );
            ui.checkbox(&mut self.show_navmesh, tr.text("tr-menu-view-show-navmesh"));
            ui.checkbox(&mut self.show_bboxes, tr.text("tr-menu-view-show-bboxes"));
            ui.checkbox(
                &mut self.show_origin_destination,
                tr.text("tr-menu-view-show-origin-destination"),
            );

            ui.separator();

            ui.checkbox(
                &mut self.show_layer_manager,
                tr.text("tr-menu-view-show-layer-manager"),
            );

            ui.separator();
            //});

            ui.label(tr.text("tr-menu-view-frame-timestep"));
            ui.add(egui::widgets::Slider::new(&mut self.frame_timestep, 0.0..=3.0).suffix(" s"));
        });
    }
}
