// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeSet, ops::ControlFlow, path::Path, sync::mpsc::Sender};

use topola::{
    autorouter::{
        anterouter::AnterouterOptions, execution::Command, invoker::InvokerError,
        multilayer_autoroute::MultilayerAutorouterOptions, selection::Selection, AutorouterOptions,
        PresortBy,
    },
    board::AccessMesadata,
    interactor::{interaction::InteractionStepper, route_plan::RoutePlan},
    router::RouterOptions,
    specctra::{design::SpecctraDesign, ParseError, ParseErrorContext as SpecctraLoadingError},
};

use crate::{
    actions::Actions,
    app::{execute, handle_file},
    error_dialog::ErrorDialog,
    translator::Translator,
    viewport::Viewport,
    workspace::Workspace,
};

pub struct MenuBar {
    pub multilayer_autorouter_options: MultilayerAutorouterOptions,
    pub is_placing_via: bool,
    pub show_ratsnest: bool,
    pub show_navmesh: bool,
    pub show_guide_circles: bool,
    pub show_guide_bitangents: bool,
    pub show_triangulation: bool,
    pub show_triangulation_constraints: bool,
    pub show_pathfinding_scores: bool,
    pub show_topo_navmesh: bool,
    pub show_bboxes: bool,
    pub show_origin_destination: bool,
    pub show_primitive_indices: bool,
    pub show_appearance_panel: bool,
    pub frame_timestep: f32,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            multilayer_autorouter_options: MultilayerAutorouterOptions {
                anterouter: AnterouterOptions {
                    fanout_clearance: 200.0,
                },
                planar: AutorouterOptions {
                    presort_by: PresortBy::RatlineIntersectionCountAndLength,
                    permutate: true,
                    router: RouterOptions {
                        routed_band_width: 100.0,
                        wrap_around_bands: true,
                        squeeze_through_under_bends: true,
                    },
                },
            },
            is_placing_via: false,
            show_ratsnest: true,
            show_navmesh: false,
            show_guide_circles: false,
            show_guide_bitangents: false,
            show_triangulation: false,
            show_triangulation_constraints: false,
            show_pathfinding_scores: false,
            show_topo_navmesh: false,
            show_bboxes: false,
            show_origin_destination: false,
            show_primitive_indices: false,
            show_appearance_panel: true,
            frame_timestep: 0.1,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        tr: &mut Translator,
        content_sender: Sender<Result<SpecctraDesign, SpecctraLoadingError>>,
        error_dialog: &mut ErrorDialog,
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
                    egui::widgets::global_theme_preference_switch(ui);
                    ui.separator();

                    ui.menu_button(tr.text("tr-menu-file"), |ui| {
                        actions.file.render_menu(ctx, ui, maybe_workspace.is_some())
                    });

                    ui.menu_button(tr.text("tr-menu-edit"), |ui| {
                        actions.edit.render_menu(
                            ctx,
                            ui,
                            maybe_workspace.is_some(),
                            workspace_activities_enabled,
                        )
                    });

                    ui.menu_button(tr.text("tr-menu-view"), |ui| {
                        actions.view.render_menu(
                            ctx,
                            ui,
                            tr,
                            self,
                            viewport,
                            maybe_workspace.is_some(),
                        );

                        ui.separator();

                        ui.label(tr.text("tr-menu-view-frame-timestep"));
                        ui.add(
                            // NOTE: Frame timestep slider's minimal value
                            // should not go down to zero seconds because this
                            // will leave no time for the GUI to update until
                            // the currently performed action finishes, which
                            // may leave the GUI unresponsive during that time,
                            // or even freeze the application if the action
                            // fails to end in reasonable time.
                            egui::widgets::Slider::new(&mut self.frame_timestep, 0.001..=3.0)
                                .suffix(" s"),
                        );
                    });

                    // NOTE: we could disable the entire range of menus below
                    // when no workspace is loaded, but that would disrupt "hover-scrolling"
                    // between menus inside of the conditionally enabled section and
                    // those outside...

                    ui.menu_button(tr.text("tr-menu-place"), |ui| {
                        actions.place.render_menu(
                            ctx,
                            ui,
                            maybe_workspace.is_some(),
                            &mut self.is_placing_via,
                        )
                    });

                    ui.menu_button(tr.text("tr-menu-route"), |ui| {
                        actions.route.render_menu(
                            ctx,
                            ui,
                            tr,
                            maybe_workspace.is_some(),
                            workspace_activities_enabled,
                            &mut self.multilayer_autorouter_options,
                        )
                    });

                    ui.menu_button(tr.text("tr-menu-inspect"), |ui| {
                        actions
                            .inspect
                            .render_menu(ctx, ui, workspace_activities_enabled)
                    });

                    Self::update_preferences_menu(ctx, ui, tr);

                    ui.menu_button(tr.text("tr-menu-help"), |ui| {
                        actions.help.render_menu(ctx, ui, online_documentation_url)
                    });

                    ui.separator();
                });

                if actions.file.open_design.consume_key_triggered(ctx, ui) {
                    // NOTE: On Linux, this requires Zenity to be installed on your system.
                    let ctx = ctx.clone();
                    let task = rfd::AsyncFileDialog::new()
                        .add_filter(tr.text("tr-menu-open-specctra-design-file"), &["dsn"])
                        .pick_file();

                    execute(async move {
                        if let Some(file_handle) = task.await {
                            let data = handle_file(&file_handle)
                                .await
                                .map_err(|e| ParseError::from(e).add_context((0, 0)))
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
                        workspace.interactor.abort();
                    } else if actions.edit.unselect_all.consume_key_triggered(ctx, ui) {
                        // NOTE: we need to check `unselect` first because `Ctrl+A` would also match `Ctrl+Shift+A`
                        workspace.overlay.unselect_all();
                    } else if actions.edit.select_all.consume_key_triggered(ctx, ui) {
                        workspace.overlay.select_all(
                            workspace.interactor.invoker().autorouter(),
                            &workspace.appearance_panel,
                        );
                    } else if actions
                        .edit
                        .recalculate_topo_navmesh
                        .consume_key_triggered(ctx, ui)
                    {
                        if let Some(active_layer) = workspace.appearance_panel.active_layer {
                            workspace.overlay.recalculate_topo_navmesh(
                                workspace.interactor.invoker().autorouter(),
                                active_layer,
                            );
                        }
                    } else if actions.place.place_via.consume_key_enabled(
                        ctx,
                        ui,
                        &mut self.is_placing_via,
                    ) {
                    } else if workspace_activities_enabled {
                        fn schedule<F: FnOnce(Selection) -> Command>(
                            error_dialog: &mut ErrorDialog,
                            workspace: &mut Workspace,
                            op: F,
                        ) {
                            let selection = workspace.overlay.take_selection();

                            if let Err(err) = workspace.interactor.schedule(op(selection)) {
                                error_dialog.push_error("tr-module-invoker", format!("{}", err));
                            }
                        }
                        if actions.edit.remove_bands.consume_key_triggered(ctx, ui) {
                            schedule(error_dialog, workspace, |selection| {
                                Command::RemoveBands(selection.band_selection)
                            })
                        } else if actions.route.topo_autoroute.consume_key_triggered(ctx, ui) {
                            if let Some(active_layer) = workspace.appearance_panel.active_layer {
                                let active_layer = workspace
                                    .interactor
                                    .invoker()
                                    .autorouter()
                                    .board()
                                    .layout()
                                    .rules()
                                    .layer_layername(active_layer)
                                    .expect("unknown active layer")
                                    .to_string();
                                schedule(error_dialog, workspace, |selection| {
                                    Command::TopoAutoroute {
                                        selection: selection.pin_selection,
                                        allowed_edges: BTreeSet::new(),
                                        active_layer,
                                        routed_band_width: self
                                            .multilayer_autorouter_options
                                            .planar
                                            .router
                                            .routed_band_width,
                                    }
                                });
                            }
                        } else if actions.route.autoroute.consume_key_triggered(ctx, ui) {
                            schedule(error_dialog, workspace, |selection| {
                                Command::MultilayerAutoroute(
                                    selection.pin_selection,
                                    self.multilayer_autorouter_options,
                                )
                            });
                        } else if actions
                            .route
                            .planar_autoroute
                            .consume_key_triggered(ctx, ui)
                        {
                            schedule(error_dialog, workspace, |selection| {
                                Command::Autoroute(
                                    selection.pin_selection,
                                    self.multilayer_autorouter_options.planar,
                                )
                            });
                        } else if actions
                            .inspect
                            .compare_detours
                            .consume_key_triggered(ctx, ui)
                        {
                            schedule(error_dialog, workspace, |selection| {
                                Command::CompareDetours(
                                    selection.pin_selection,
                                    self.multilayer_autorouter_options.planar,
                                )
                            });
                        } else if actions
                            .inspect
                            .measure_length
                            .consume_key_triggered(ctx, ui)
                        {
                            schedule(error_dialog, workspace, |selection| {
                                Command::MeasureLength(selection.band_selection)
                            });
                        } else if actions
                            .place
                            .place_route_plan
                            .consume_key_triggered(ctx, ui)
                        {
                            if let Some(active_layer) = workspace.appearance_panel.active_layer {
                                self.is_placing_via = false;
                                workspace.interactor.interact(InteractionStepper::RoutePlan(
                                    RoutePlan::new(active_layer),
                                ));
                            }
                        }
                    }
                }
                Ok::<(), InvokerError>(())
            })
            .inner
    }

    pub fn update_preferences_menu(_ctx: &egui::Context, ui: &mut egui::Ui, tr: &mut Translator) {
        ui.menu_button(tr.text("tr-menu-preferences"), |ui| {
            ui.menu_button(tr.text("tr-menu-preferences-set-language"), |ui| {
                use icu_experimental::displaynames::{
                    DisplayNamesOptions, Fallback, LocaleDisplayNamesFormatter,
                };
                use icu_locale_core::Locale;

                let mut display_names_options: DisplayNamesOptions = Default::default();
                display_names_options.fallback = Fallback::None;

                for langid in Translator::locales() {
                    if let Ok(locale) = Locale::try_from_str(&langid.to_string()) {
                        if let Ok(formatter) = LocaleDisplayNamesFormatter::try_new(
                            locale.clone().into(),
                            display_names_options,
                        ) {
                            // NOTE: I don't know how to reliably detect if there's no display name
                            // in the current locale to fall back to the English display name.
                            // NOTE: At the time of writing, `Fallback::None` handling hasn't been
                            // implemented in the ICU library, despite this enum variant existing.
                            ui.radio_value(tr.langid_mut(), langid.clone(), formatter.of(&locale));
                            continue;
                        }
                    }

                    ui.radio_value(tr.langid_mut(), langid.clone(), langid.to_string());
                }
            });
        });
    }
}
