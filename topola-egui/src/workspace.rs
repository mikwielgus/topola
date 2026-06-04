// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{ops::ControlFlow, time::Instant};

use topola::{Board, Workspace};

use crate::{layers_panel::LayersPanel, translator::Translator};

pub struct GuiWorkspace {
    pub workspace: Workspace,
    pub appearance_panel: LayersPanel,
    pub dt_accum: f64,
}

impl GuiWorkspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = LayersPanel::new(&board);

        Self {
            workspace: Workspace::new_board(board),
            appearance_panel,
            dt_accum: 0.0,
        }
    }

    pub fn advance_state_by_dt(
        &mut self,
        tr: &Translator,
        step_rate: Option<f64>,
        dt: f64,
    ) -> bool {
        let instant = Instant::now();

        if step_rate.is_some() {
            self.dt_accum += dt;
        }

        while step_rate.is_none_or(|step_rate| self.dt_accum >= 1.0 / step_rate) {
            if let Some(step_rate) = step_rate {
                self.dt_accum -= 1.0 / step_rate;
            }

            if let ControlFlow::Break(()) = self.step(tr) {
                return true;
            }

            // Hard limit: never spend more time on advancing state than the
            // duration of last frame to prevent stuttering.
            // Of course, this does not safeguard against infinite loops.
            if instant.elapsed().as_secs_f64() >= dt {
                return false;
            }
        }

        true
    }

    pub fn step(&mut self, tr: &Translator) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        let Self {
            workspace,
            appearance_panel,
            dt_accum,
        } = self;

        appearance_panel.update(ctx, workspace.board());
    }
}
