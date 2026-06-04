// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    board::{
        Board,
        interactors::SelectInteractor,
        selections::PersistableSelection,
    },
    layout::LayerId,
    vector::Vector2,
};

pub trait Interactor {
    fn delete(&mut self, board: &mut Board);
    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>);
    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>);
    fn abort(&mut self, board: &mut Board);
}

pub enum MasterInteractor {
    Board(crate::board::interactors::MasterInteractor),
}

impl MasterInteractor {
    pub fn new(selection: PersistableSelection) -> Self {
        Self::Board(crate::board::interactors::MasterInteractor::new(selection))
    }

    pub fn selection(&self) -> &PersistableSelection {
        match self {
            Self::Board(interactor) => interactor.selection(),
        }
    }

    pub fn select_interactor(&self) -> &Option<SelectInteractor> {
        match self {
            Self::Board(interactor) => interactor.select_interactor(),
        }
    }
}

impl Interactor for MasterInteractor {
    fn delete(&mut self, board: &mut Board) {
        match self {
            Self::Board(interactor) => interactor.delete(board),
        }
    }

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        match self {
            Self::Board(interactor) => interactor.hold(board, layer, pointer),
        }
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        match self {
            Self::Board(interactor) => interactor.release(board, layer, pointer),
        }
    }

    fn abort(&mut self, board: &mut Board) {
        match self {
            Self::Board(interactor) => interactor.abort(board),
        }
    }
}
