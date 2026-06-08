// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::ControlFlow;

use derive_getters::Getters;
use derive_more::Constructor;
use undoredo::ResetDelta;

use crate::{
    board::Board, interactor::Interactor, layout::LayerId, selections::ComponentSelection,
    vector::Vector2,
};

#[derive(Clone, Constructor, Debug, Eq, Getters, PartialEq)]
pub struct DragMoveInteractor {
    origin: Vector2<i64>,
    layer: LayerId,
    selection: ComponentSelection,
}

impl Interactor for DragMoveInteractor {
    fn hold(&mut self, board: &mut Board, _layer: LayerId, pointer: Vector2<i64>) {
        board.reset_delta();

        board.move_components_by(self.selection.clone(), pointer - self.origin);
    }

    fn release(&mut self, board: &mut Board, layer: LayerId, pointer: Vector2<i64>) -> ControlFlow<()> {
        self.hold(board, layer, pointer);
        ControlFlow::Break(())
    }

    fn abort(&mut self, board: &mut Board) {
        board.reset_delta();
    }
}
