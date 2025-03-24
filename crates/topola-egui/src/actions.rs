// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::{
    action::{Action, Switch, Trigger},
    menu_bar::MenuBar,
    translator::Translator,
    viewport::Viewport,
};

use egui::{Context, Ui};
use topola::autorouter::AutorouterOptions;

pub struct FileActions {
    pub open_design: Trigger,
    pub export_session: Trigger,
    pub import_history: Trigger,
    pub export_history: Trigger,
    pub quit: Trigger,
}

impl FileActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            open_design: Action::new(
                tr.text("tr-menu-file-open"),
                egui::Modifiers::CTRL,
                egui::Key::O,
            )
            .into_trigger(),
            export_session: Action::new(
                tr.text("tr-menu-file-export-session-file"),
                egui::Modifiers::CTRL,
                egui::Key::S,
            )
            .into_trigger(),
            import_history: Action::new(
                tr.text("tr-menu-file-import-history"),
                egui::Modifiers::CTRL,
                egui::Key::I,
            )
            .into_trigger(),
            export_history: Action::new(
                tr.text("tr-menu-file-export-history"),
                egui::Modifiers::CTRL,
                egui::Key::E,
            )
            .into_trigger(),
            quit: Action::new(
                tr.text("tr-menu-file-quit"),
                egui::Modifiers::CTRL,
                egui::Key::Q,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui, have_workspace: bool) {
        self.open_design.button(ctx, ui);
        ui.add_enabled_ui(have_workspace, |ui| {
            self.export_session.button(ctx, ui);

            ui.separator();

            self.import_history.button(ctx, ui);
            self.export_history.button(ctx, ui);
        });

        ui.separator();

        // "Quit" button wouldn't work on a Web page.
        if !cfg!(target_arch = "wasm32") {
            self.quit.button(ctx, ui);
        }
    }
}

pub struct EditActions {
    pub undo: Trigger,
    pub redo: Trigger,
    pub abort: Trigger,
    pub select_all: Trigger,
    pub unselect_all: Trigger,
    pub recalculate_topo_navmesh: Trigger,
    pub remove_bands: Trigger,
}

impl EditActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            undo: Action::new(
                tr.text("tr-menu-edit-undo"),
                egui::Modifiers::CTRL,
                egui::Key::Z,
            )
            .into_trigger(),
            redo: Action::new(
                tr.text("tr-menu-edit-redo"),
                egui::Modifiers::CTRL,
                egui::Key::Y,
            )
            .into_trigger(),
            abort: Action::new(
                tr.text("tr-menu-edit-abort"),
                egui::Modifiers::NONE,
                egui::Key::Escape,
            )
            .into_trigger(),
            select_all: Action::new(
                tr.text("tr-menu-edit-select-all"),
                egui::Modifiers::CTRL,
                egui::Key::A, // taken from KiCAD
            )
            .into_trigger(),
            unselect_all: Action::new(
                tr.text("tr-menu-edit-unselect-all"),
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::A,
            )
            .into_trigger(),
            recalculate_topo_navmesh: Action::new_keyless(
                tr.text("tr-menu-edit-recalculate-topo-navmesh"),
            )
            .into_trigger(),
            remove_bands: Action::new(
                tr.text("tr-menu-edit-remove-bands"),
                egui::Modifiers::NONE,
                egui::Key::Delete,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        have_workspace: bool,
        workspace_activities_enabled: bool,
    ) -> egui::InnerResponse<()> {
        ui.add_enabled_ui(have_workspace, |ui| {
            self.undo.button(ctx, ui);
            self.redo.button(ctx, ui);

            ui.separator();

            self.abort.button(ctx, ui);

            ui.separator();

            self.select_all.button(ctx, ui);
            self.unselect_all.button(ctx, ui);
            self.recalculate_topo_navmesh.button(ctx, ui);

            ui.separator();

            ui.add_enabled_ui(workspace_activities_enabled, |ui| {
                self.remove_bands.button(ctx, ui);
            });
        })
    }
}

pub struct ViewActions {
    pub zoom_to_fit: Switch,
    pub show_ratsnest: Switch,
    pub show_navmesh: Switch,
    pub show_pathfinding_scores: Switch,
    pub show_topo_navmesh: Switch,
    pub show_bboxes: Switch,
    pub show_origin_destination: Switch,
    pub show_appearance_panel: Switch,
}

impl ViewActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            zoom_to_fit: Action::new_keyless(tr.text("tr-menu-view-zoom-to-fit")).into_switch(),
            show_ratsnest: Action::new_keyless(tr.text("tr-menu-view-show-ratsnest")).into_switch(),
            show_navmesh: Action::new_keyless(tr.text("tr-menu-view-show-navmesh")).into_switch(),
            show_pathfinding_scores: Action::new_keyless(
                tr.text("tr-menu-view-show-pathfinding-scores"),
            )
            .into_switch(),
            show_topo_navmesh: Action::new_keyless(tr.text("tr-menu-view-show-topo-navmesh"))
                .into_switch(),
            show_bboxes: Action::new_keyless(tr.text("tr-menu-view-show-bboxes")).into_switch(),
            show_origin_destination: Action::new_keyless(
                tr.text("tr-menu-view-show-origin-destination"),
            )
            .into_switch(),
            show_appearance_panel: Action::new_keyless(tr.text("tr-menu-view-show-layer-manager"))
                .into_switch(),
        }
    }

    pub fn render_menu(
        &mut self,
        _ctx: &Context,
        ui: &mut Ui,
        tr: &Translator,
        menu_bar: &mut MenuBar,
        viewport: &mut Viewport,
        have_workspace: bool,
    ) {
        self.zoom_to_fit
            .toggle_widget(ui, &mut viewport.scheduled_zoom_to_fit);

        ui.separator();
        ui.add_enabled_ui(have_workspace, |ui| {
            self.show_ratsnest.checkbox(ui, &mut menu_bar.show_ratsnest);
            self.show_navmesh.checkbox(ui, &mut menu_bar.show_navmesh);
            self.show_pathfinding_scores
                .checkbox(ui, &mut menu_bar.show_pathfinding_scores);
            self.show_topo_navmesh
                .checkbox(ui, &mut menu_bar.show_topo_navmesh);
            self.show_bboxes.checkbox(ui, &mut menu_bar.show_bboxes);
            self.show_origin_destination
                .checkbox(ui, &mut menu_bar.show_origin_destination);
        });

        ui.separator();
        self.show_appearance_panel
            .checkbox(ui, &mut menu_bar.show_appearance_panel);

        ui.separator();
        ui.label(tr.text("tr-menu-view-kdb-scroll-delta-factor"));
        ui.add(egui::widgets::Slider::new(
            &mut viewport.kbd_scroll_delta_factor,
            1.0..=100.0,
        ));
    }
}

pub struct PlaceActions {
    pub place_via: Switch,
    pub place_route_plan: Trigger,
}

impl PlaceActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            place_via: Action::new(
                tr.text("tr-menu-place-place-via"),
                egui::Modifiers::CTRL,
                egui::Key::P,
            )
            .into_switch(),
            place_route_plan: Action::new_keyless(tr.text("tr-menu-place-place-route-plan"))
                .into_trigger(),
        }
    }

    pub fn render_menu(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        have_workspace: bool,
        is_placing_via: &mut bool,
    ) -> egui::InnerResponse<()> {
        ui.add_enabled_ui(have_workspace, |ui| {
            self.place_via.toggle_widget(ui, is_placing_via);
            self.place_route_plan.button(ctx, ui);
        })
    }
}

pub struct RouteActions {
    pub autoroute: Trigger,
    pub topo_autoroute: Trigger,
}

impl RouteActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            autoroute: Action::new(
                tr.text("tr-menu-route-autoroute"),
                egui::Modifiers::CTRL,
                egui::Key::R,
            )
            .into_trigger(),
            topo_autoroute: Action::new(
                tr.text("tr-menu-route-topo-autoroute"),
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::R,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        tr: &Translator,
        have_workspace: bool,
        workspace_activities_enabled: bool,
        autorouter_options: &mut AutorouterOptions,
    ) -> egui::InnerResponse<()> {
        ui.add_enabled_ui(have_workspace, |ui| {
            ui.add_enabled_ui(workspace_activities_enabled, |ui| {
                self.autoroute.button(ctx, ui);
                self.topo_autoroute.button(ctx, ui);
            });
            ui.separator();

            ui.label(tr.text("tr-menu-route-routed-band-width"));

            ui.add(
                egui::widgets::Slider::new(
                    &mut autorouter_options.router_options.routed_band_width,
                    1.0..=1000.0,
                )
                .suffix(""),
            );

            ui.separator();

            ui.menu_button(tr.text("tr-menu-options"), |ui| {
                ui.checkbox(
                    &mut autorouter_options.presort_by_pairwise_detours,
                    tr.text("tr-menu-route-options-presort-by-pairwise-detours"),
                );
                ui.checkbox(
                    &mut autorouter_options
                        .router_options
                        .squeeze_through_under_bends,
                    tr.text("tr-menu-route-options-squeeze-through-under-bends"),
                );
                ui.checkbox(
                    &mut autorouter_options.router_options.wrap_around_bands,
                    tr.text("tr-menu-route-options-wrap-around-bands"),
                );
            });
        })
    }
}

pub struct InspectActions {
    pub compare_detours: Trigger,
    pub measure_length: Trigger,
}

impl InspectActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            compare_detours: Action::new(
                tr.text("tr-menu-inspect-compare-detours"),
                egui::Modifiers::NONE,
                egui::Key::Minus,
            )
            .into_trigger(),
            measure_length: Action::new(
                tr.text("tr-menu-inspect-measure-length"),
                egui::Modifiers::NONE,
                egui::Key::Plus,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui, workspace_activities_enabled: bool) {
        ui.add_enabled_ui(workspace_activities_enabled, |ui| {
            self.compare_detours.button(ctx, ui);
            self.measure_length.button(ctx, ui);
        });
    }
}

pub struct HelpActions {
    pub online_documentation: Trigger,
}

impl HelpActions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            online_documentation: Action::new(
                tr.text("tr-menu-help-online-documentation"),
                egui::Modifiers::NONE,
                egui::Key::F1,
            )
            .into_trigger(),
        }
    }

    pub fn render_menu(&mut self, ctx: &Context, ui: &mut Ui, online_documentation_url: &str) {
        self.online_documentation
            .hyperlink(ctx, ui, online_documentation_url);
    }
}

pub struct Actions {
    pub file: FileActions,
    pub edit: EditActions,
    pub view: ViewActions,
    pub place: PlaceActions,
    pub route: RouteActions,
    pub inspect: InspectActions,
    pub help: HelpActions,
}

impl Actions {
    pub fn new(tr: &Translator) -> Self {
        Self {
            file: FileActions::new(tr),
            edit: EditActions::new(tr),
            view: ViewActions::new(tr),
            place: PlaceActions::new(tr),
            route: RouteActions::new(tr),
            inspect: InspectActions::new(tr),
            help: HelpActions::new(tr),
        }
    }
}
