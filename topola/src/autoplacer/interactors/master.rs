// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    autoplacer::{Autoplacer, AutoplacerSchedule},
    board::{
        Board,
        interactors::{MasterInteractor as BoardMasterInteractor, SelectInteractor},
        selections::PersistableSelection,
    },
    interactor::Interactor,
    layout::LayerId,
    vector::Vector2,
};

pub struct MasterInteractor {
    board_master: BoardMasterInteractor,
    autoplacer: Autoplacer,
}

impl MasterInteractor {
    pub fn new(
        board: &mut Board,
        selection: PersistableSelection,
        schedule: AutoplacerSchedule,
    ) -> Self {
        Self {
            board_master: BoardMasterInteractor::new(selection.clone()),
            autoplacer: Autoplacer::new(board, selection.components.clone(), schedule),
        }
    }

    pub fn selection(&self) -> &PersistableSelection {
        self.board_master.selection()
    }

    pub fn select_interactor(&self) -> &Option<SelectInteractor> {
        self.board_master.select_interactor()
    }
}

impl Interactor for MasterInteractor {
    fn step(&mut self, board: &mut Board) {
        self.autoplacer.step(board);
    }

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        self.board_master.hold(board, layer, pointer);
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        self.board_master.release(board, layer, pointer);
    }

    fn abort(&mut self, board: &mut Board) {
        self.board_master.abort(board);
    }
}
