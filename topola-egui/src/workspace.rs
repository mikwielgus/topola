// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use topola::{Autorouter, Board, selections::PersistableSelection};

use crate::{layers_panel::LayersPanel, translator::Translator};

pub struct Workspace {
    pub autorouter: Autorouter,
    pub appearance_panel: LayersPanel,
    pub selection: PersistableSelection,
}

impl Workspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = LayersPanel::new(&board);

        Self {
            autorouter: Autorouter::new(board),
            appearance_panel,
            selection: PersistableSelection::new(),
        }
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        self.appearance_panel
            .update(ctx, &self.autorouter.router().navmesher_board().board());
    }
}
