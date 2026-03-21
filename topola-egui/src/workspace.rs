// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use topola::{Autorouter, Board, PinSelection};

use crate::{appearance_panel::AppearancePanel, translator::Translator};

pub struct Workspace {
    pub autorouter: Autorouter,
    pub appearance_panel: AppearancePanel,
    pub pin_selection: PinSelection,
}

impl Workspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = AppearancePanel::new(&board);

        Self {
            autorouter: Autorouter::new(board),
            appearance_panel,
            pin_selection: PinSelection::new(),
        }
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        self.appearance_panel
            .update(ctx, &self.autorouter.router().navmesher_board().board());
    }
}
