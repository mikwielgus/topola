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
    selection: PersistableSelection,
    selection_interactor: Option<SelectionInteractor>,
}

impl MasterInteractor {
    pub fn update(&mut self, board: &mut Board, input: InteractiveInput) {
        if input.delete {
            board.delete_net_free_primitives(self.selection.nets.clone());
        }

        if self.selection_interactor.is_none() {
            self.selection_interactor = Some(SelectionInteractor::new(
                input.pointer,
                self.selection.clone(),
                SelectionCombineMode::Additive,
            ));
        }

        if let Some(selection_interactor) = self.selection_interactor.as_mut() {
            if let Some(selection) =
                selection_interactor.update(board, LayerId::new(0), input.clone())
            {
                self.selection = selection;
            }
        }

        if input.release || input.cancel {
            self.selection_interactor = None;
        }
    }
}
