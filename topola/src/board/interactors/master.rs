// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use derive_more::Constructor;

use crate::{
    InteractiveInput,
    board::{
        Board,
        interactors::{SelectionCombineMode, SelectionInteractor},
        selections::PersistableSelection,
    },
    layout::LayerId,
};

#[derive(Clone, Constructor, Debug, Eq, Getters, PartialEq)]
pub struct MasterInteractor {
    selection_interactor: Option<SelectionInteractor>,
    selection: PersistableSelection,
}

impl MasterInteractor {
    pub fn update(&mut self, board: &mut Board, layer: LayerId, input: InteractiveInput) {
        if input.delete {
            board.delete_net_free_primitives(self.selection.nets.clone());
        }

        if self.selection_interactor.is_none() {
            self.selection_interactor = Some(SelectionInteractor::new(
                input.pointer,
                self.selection.clone(),
                SelectionCombineMode::Replace,
            ));
        }

        if let Some(selection_interactor) = self.selection_interactor.as_mut() {
            selection_interactor.update(board, layer, input.clone());
            self.selection = selection_interactor.selection().clone();
        }

        if input.release || input.cancel {
            self.selection_interactor = None;
        }
    }
}
