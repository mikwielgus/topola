// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use undoredo::{FlushDelta, UndoRedo};

use crate::{
    board::{Board, BoardDelta, selections::PersistableSelection},
    ratsnest::Ratsnest,
};

#[derive(Getters)]
pub struct Workspace {
    board: Board,
    selection: PersistableSelection,
    history: UndoRedo<BoardDelta>,
    ratsnest: Ratsnest,
}

impl Workspace {
    pub fn new(mut board: Board) -> Self {
        board.flush_delta();
        let ratsnest = Ratsnest::new(board.layout());

        Self {
            board,
            selection: PersistableSelection::new(),
            history: UndoRedo::new(),
            ratsnest,
        }
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn selection_mut(&mut self) -> &mut PersistableSelection {
        &mut self.selection
    }

    pub fn commit(&mut self) {
        self.history.commit(&mut self.board);
        self.rebuild_ratsnest();
    }

    pub fn undo(&mut self) -> bool {
        if self.history.undo(&mut self.board).is_some() {
            self.rebuild_ratsnest();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.history.redo(&mut self.board).is_some() {
            self.rebuild_ratsnest();
            true
        } else {
            false
        }
    }

    fn rebuild_ratsnest(&mut self) {
        self.ratsnest = Ratsnest::new(self.board.layout());
    }
}
