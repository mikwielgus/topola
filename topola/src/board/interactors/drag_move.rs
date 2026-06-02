// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;
use derive_more::Constructor;
use undoredo::DiscardDelta;

use crate::{Board, LayerId, Vector2, selections::ComponentSelection};

#[derive(Clone, Constructor, Debug, Eq, Getters, PartialEq)]
pub struct DragMoveInteractor {
    origin: Vector2<i64>,
    layer: LayerId,
    selection: ComponentSelection,
}

impl DragMoveInteractor {
    pub fn hold(&mut self, board: &mut Board, pointer: Vector2<i64>) {
        board.discard_delta();

        board.move_components_by(self.selection.clone(), pointer - self.origin);
    }

    pub fn abort(&mut self, board: &mut Board) {
        board.discard_delta();
    }

    pub fn release(&mut self, board: &mut Board, pointer: Vector2<i64>) {
        self.hold(board, pointer);
    }
}
