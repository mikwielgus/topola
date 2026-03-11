// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use topola::{Board, NavmesherBoard};

use crate::{appearance_panel::AppearancePanel, translator::Translator};

pub struct Workspace {
    pub navmesher_board: NavmesherBoard,
    pub appearance_panel: AppearancePanel,
}

impl Workspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        let appearance_panel = AppearancePanel::new(&board);

        Self {
            navmesher_board: NavmesherBoard::with_board(board),
            appearance_panel,
        }
    }

    pub fn update_appearance_panel(&mut self, ctx: &egui::Context) {
        self.appearance_panel
            .update(ctx, &self.navmesher_board.board());
    }
}
