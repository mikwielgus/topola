// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::ControlFlow;

use crate::{
    autoplacer::{AutoplacerSchedule, interactors::AutoplacerMasterInteractor},
    board::{
        Board,
        interactors::{BoardMasterInteractor, SelectInteractor},
        selections::PersistableSelection,
    },
    layout::LayerId,
    vector::Vector2,
};

pub trait Interactor {
    fn step(&mut self, _board: &mut Board) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
    fn delete(&mut self, board: &mut Board) {}
    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {}
    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {}
    fn abort(&mut self, board: &mut Board) {}
}

pub enum MasterInteractor {
    Board(BoardMasterInteractor),
    Autoplacer(AutoplacerMasterInteractor),
}

impl MasterInteractor {
    pub fn new(selection: PersistableSelection) -> Self {
        Self::Board(crate::board::interactors::BoardMasterInteractor::new(
            selection,
        ))
    }

    pub fn autoplace(&mut self, board: &mut Board, schedule: AutoplacerSchedule) {
        match self {
            Self::Board(board_master) => {
                *self = Self::Autoplacer(AutoplacerMasterInteractor::new(
                    board,
                    board_master.clone(),
                    schedule,
                ));
            }
            _ => (),
            //_ => panic!("autoplacement can be only started from board at rest"),
        }
    }

    pub fn selection(&self) -> &PersistableSelection {
        match self {
            Self::Board(board_master) => board_master.selection(),
            Self::Autoplacer(autoplacer_master) => autoplacer_master.selection(),
        }
    }

    pub fn select_interactor(&self) -> &Option<SelectInteractor> {
        match self {
            Self::Board(board_master) => board_master.select_interactor(),
            Self::Autoplacer(autoplacer_master) => autoplacer_master.select_interactor(),
        }
    }
}

impl Interactor for MasterInteractor {
    fn step(&mut self, board: &mut Board) -> ControlFlow<()> {
        match self {
            Self::Board(interactor) => interactor.step(board),
            Self::Autoplacer(interactor) => {
                if interactor.step(board).is_break() {
                    *self = Self::Board(interactor.board_master().clone());
                }

                ControlFlow::Continue(())
            }
        }
    }

    fn delete(&mut self, board: &mut Board) {
        match self {
            Self::Board(interactor) => interactor.delete(board),
            Self::Autoplacer(interactor) => interactor.delete(board),
        }
    }

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        match self {
            Self::Board(interactor) => interactor.hold(board, layer, pointer),
            Self::Autoplacer(interactor) => interactor.hold(board, layer, pointer),
        }
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        match self {
            Self::Board(interactor) => interactor.release(board, layer, pointer),
            Self::Autoplacer(interactor) => interactor.release(board, layer, pointer),
        }
    }

    fn abort(&mut self, board: &mut Board) {
        match self {
            Self::Board(interactor) => interactor.abort(board),
            Self::Autoplacer(interactor) => interactor.abort(board),
        }
    }
}
