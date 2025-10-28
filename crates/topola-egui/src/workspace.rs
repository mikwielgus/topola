// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{
    ops::ControlFlow,
    sync::mpsc::{channel, Receiver, Sender},
    time::Instant,
};

use topola::{
    autorouter::{
        execution::Command, history::History, multilayer_autoroute::MultilayerAutorouteOptions,
    },
    board::edit::BoardEdit,
    interactor::{
        activity::{InteractiveEvent, InteractiveEventKind, InteractiveInput},
        Interactor,
    },
    layout::via::ViaWeight,
    math::Circle,
    specctra::{design::SpecctraDesign, mesadata::SpecctraMesadata},
};

use crate::{
    appearance_panel::AppearancePanel, error_dialog::ErrorDialog, menu_bar::MenuBar,
    overlay::Overlay, translator::Translator,
};

/// A loaded design and associated structures.
pub struct Workspace {
    pub design: SpecctraDesign,
    pub appearance_panel: AppearancePanel,
    pub overlay: Overlay,
    pub interactor: Interactor<SpecctraMesadata>,

    pub history_channel: (
        Sender<std::io::Result<Result<History, serde_json::Error>>>,
        Receiver<std::io::Result<Result<History, serde_json::Error>>>,
    ),

    update_counter: f32,
}

impl Workspace {
    pub fn new(design: SpecctraDesign, tr: &Translator) -> Result<Self, String> {
        let board = design.make_board(&mut BoardEdit::new());
        let appearance_panel = AppearancePanel::new(&board);
        let overlay = Overlay::new(&board).map_err(|err| {
            format!(
                "{}; {}",
                tr.text("tr-error-unable-to-initialize-overlay"),
                err
            )
        })?;
        Ok(Self {
            design,
            appearance_panel,
            overlay,
            interactor: Interactor::new(board).map_err(|err| {
                format!(
                    "{}; {}",
                    tr.text("tr-error-unable-to-initialize-overlay"),
                    err
                )
            })?,
            history_channel: channel(),
            update_counter: 0.0,
        })
    }

    /// Advances the app's state by the delta time `dt`. May call
    /// `.update_state()` more than once if the delta time is more than a multiple of
    /// the timestep.
    pub fn advance_state_by_dt(
        &mut self,
        tr: &Translator,
        error_dialog: &mut ErrorDialog,
        frame_timestep: f32,
        interactive_input: &InteractiveInput,
    ) -> bool {
        let instant = Instant::now();

        self.update_counter += interactive_input.dt;
        while self.update_counter >= frame_timestep {
            self.update_counter -= frame_timestep;

            if let ControlFlow::Break(()) = self.update_state(tr, error_dialog, interactive_input) {
                return true;
            }

            // Hard limit: never spend more time on advancing state than the
            // duration of last frame to prevent stuttering.
            // Of course, this does not safeguard against infinite loops.
            if instant.elapsed().as_secs_f32() >= interactive_input.dt {
                return false;
            }
        }

        true
    }

    pub fn update_state_for_event(
        &mut self,
        tr: &Translator,
        error_dialog: &mut ErrorDialog,
        menu_bar: &MenuBar,
        interactive_input: &InteractiveInput,
        interactive_event: InteractiveEvent,
    ) {
        if !self
            .interactor
            .maybe_activity()
            .as_ref()
            .map_or(true, |activity| {
                matches!(activity.maybe_status(), Some(ControlFlow::Break(..)))
            })
        {
            match self
                .interactor
                .update_for_event(interactive_input, interactive_event)
            {
                ControlFlow::Continue(()) | ControlFlow::Break(Ok(())) => {}
                ControlFlow::Break(Err(err)) => {
                    error_dialog.push_error("tr-module-invoker", format!("{}", err));
                }
            }
        } else {
            match interactive_event.kind {
                InteractiveEventKind::PointerPrimaryButtonClicked => {
                    if menu_bar.is_placing_via {
                        self.interactor.execute(Command::PlaceVia(ViaWeight {
                            from_layer: 0,
                            to_layer: 0,
                            circle: Circle {
                                pos: interactive_input.pointer_pos,
                                r: menu_bar
                                    .multilayer_autoroute_options
                                    .planar
                                    .router
                                    .routed_band_width
                                    / 2.0,
                            },
                            maybe_net: Some(1234),
                        }));
                    } else {
                        self.overlay.click(
                            self.interactor.invoker().autorouter(),
                            &self.appearance_panel,
                            interactive_input.pointer_pos,
                        );
                    }
                }
                InteractiveEventKind::PointerPrimaryButtonDragStarted => {
                    self.overlay.drag_start(
                        self.interactor.invoker().autorouter(),
                        &self.appearance_panel,
                        interactive_input.pointer_pos,
                        interactive_event.ctrl,
                        interactive_event.shift,
                    );
                }
                InteractiveEventKind::PointerPrimaryButtonDragStopped => {
                    self.overlay.drag_stop(
                        self.interactor.invoker().autorouter(),
                        &self.appearance_panel,
                        interactive_input.pointer_pos,
                    );
                }
                _ => {}
            }
        }
    }

    pub fn update_state(
        &mut self,
        tr: &Translator,
        error_dialog: &mut ErrorDialog,
        interactive_input: &InteractiveInput,
    ) -> ControlFlow<()> {
        if let Ok(data) = self.history_channel.1.try_recv() {
            match data {
                Ok(Ok(data)) => {
                    self.interactor.replay(data);
                }
                Ok(Err(err)) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!(
                            "{}; {}",
                            tr.text("tr-error-failed-to-parse-as-history-json"),
                            err
                        ),
                    );
                }
                Err(err) => {
                    error_dialog.push_error(
                        "tr-module-history-file-loader",
                        format!("{}; {}", tr.text("tr-error-unable-to-read-file"), err),
                    );
                }
            }
        }

        match self.interactor.update(interactive_input) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(Ok(())) => ControlFlow::Break(()),
            ControlFlow::Break(Err(err)) => {
                error_dialog.push_error("tr-module-invoker", format!("{}", err));
                ControlFlow::Break(())
            }
        }
    }

    pub fn update_appearance_panel(
        &mut self,
        ctx: &egui::Context,
        options: &mut MultilayerAutorouteOptions,
    ) {
        self.appearance_panel
            .update(ctx, self.interactor.invoker().autorouter().board(), options);
    }
}
