// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use topola::{Board, Workspace};

use crate::{layers_panel::LayersPanel, translator::Translator};

pub struct GuiWorkspace {
    pub workspace: Workspace,
    pub appearance_panel: LayersPanel,
}

impl GuiWorkspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = LayersPanel::new(&board);

        Self {
            workspace: Workspace::new_board(board),
            appearance_panel,
        }
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        let Self {
            workspace,
            appearance_panel,
        } = self;
        appearance_panel.update(ctx, workspace.board());
    }
}
