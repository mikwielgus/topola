// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::ControlFlow;

use derive_getters::Getters;

use crate::{
    autoplacer::{Autoplacer, AutoplacerSchedule},
    board::{
        Board,
        interactors::{BoardMasterInteractor, SelectInteractor},
        selections::PersistableSelection,
    },
    interactor::Interactor,
    layout::LayerId,
    vector::Vector2,
};

#[derive(Getters)]
pub struct AutoplacerMasterInteractor {
    board_master: BoardMasterInteractor,
    autoplacer: Autoplacer,
}

impl AutoplacerMasterInteractor {
    pub fn new(
        board: &mut Board,
        board_master: BoardMasterInteractor,
        schedule: AutoplacerSchedule,
    ) -> Self {
        let selection = board_master.selection().components.clone();

        Self {
            board_master,
            autoplacer: Autoplacer::new(board, selection, schedule),
        }
    }

    pub fn selection(&self) -> &PersistableSelection {
        self.board_master.selection()
    }

    pub fn select_interactor(&self) -> &Option<SelectInteractor> {
        self.board_master.select_interactor()
    }
}

impl Interactor for AutoplacerMasterInteractor {
    fn step(&mut self, board: &mut Board) -> ControlFlow<()> {
        crate::profile_function!();
        self.autoplacer.step(board)
    }

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        self.board_master.hold(board, layer, pointer);
    }

    fn release(
        &mut self,
        board: &mut Board,
        layer: LayerId,
        pointer: Vector2<i64>,
    ) -> ControlFlow<()> {
        self.board_master.release(board, layer, pointer)
    }

    fn abort(&mut self, board: &mut Board) {
        self.board_master.abort(board);
    }
}
