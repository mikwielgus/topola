// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{
    board::{
        Board,
        interactors::{DragMoveInteractor, SelectInteractor, SelectionCombineMode},
        selections::PersistableSelection,
    },
    interactor::Interactor,
    layout::LayerId,
    vector::Vector2,
};

#[derive(Clone, Debug, Eq, Getters, PartialEq)]
pub struct MasterInteractor {
    select_interactor: Option<SelectInteractor>,
    drag_move_interactor: Option<DragMoveInteractor>,
    selection: PersistableSelection,
}

impl MasterInteractor {
    pub fn new(selection: PersistableSelection) -> Self {
        Self {
            select_interactor: None,
            drag_move_interactor: None,
            selection,
        }
    }
}

impl Interactor for MasterInteractor {
    fn delete(&mut self, board: &mut Board) {
        board.delete_net_free_primitives(self.selection.nets.clone());
    }

    fn hold(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        if self.select_interactor.is_none() && self.drag_move_interactor.is_none() {
            if board.components_contain_point(&self.selection.components, pointer) {
                self.drag_move_interactor = Some(DragMoveInteractor::new(
                    pointer,
                    layer,
                    self.selection.components.clone(),
                ));
            } else {
                self.select_interactor = Some(SelectInteractor::new(
                    pointer,
                    self.selection.clone(),
                    SelectionCombineMode::Replace,
                ));
            }
        }

        if let Some(drag_move_interactor) = self.drag_move_interactor.as_mut() {
            drag_move_interactor.hold(board, layer, pointer);
        } else if let Some(select_interactor) = self.select_interactor.as_mut() {
            select_interactor.hold(board, layer, pointer);
            self.selection = select_interactor.selection().clone();
        }
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) {
        if let Some(drag_move_interactor) = self.drag_move_interactor.as_mut() {
            drag_move_interactor.release(board, layer, pointer);
        } else if let Some(select_interactor) = self.select_interactor.as_mut() {
            select_interactor.release(board, layer, pointer);
            self.selection = select_interactor.selection().clone();
        }

        self.select_interactor = None;
        self.drag_move_interactor = None;
    }

    fn abort(&mut self, board: &mut Board) {
        if let Some(drag_move_interactor) = self.drag_move_interactor.as_mut() {
            drag_move_interactor.abort(board);
        }

        if let Some(select_interactor) = self.select_interactor.as_mut() {
            select_interactor.abort(board);
            self.selection = select_interactor.original_selection().clone();
        }

        self.select_interactor = None;
        self.drag_move_interactor = None;
    }
}
