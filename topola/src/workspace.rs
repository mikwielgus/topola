// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use undoredo::FlushDelta;

use crate::{autorouter::Autorouter, board::Board, selections::PersistableSelection};

pub enum Workspace {
    Board(BoardWorkspace),
    Autorouter(AutorouterWorkspace),
}

impl Workspace {
    pub fn new_board(mut board: Board) -> Self {
        board.flush_delta();

        Self::Board(BoardWorkspace::new(board))
    }

    pub fn new_autorouter(board: Board) -> Self {
        Self::Autorouter(AutorouterWorkspace::new(board))
    }

    pub fn selection(&self) -> &PersistableSelection {
        match self {
            Workspace::Board(workspace) => &workspace.selection,
            Workspace::Autorouter(workspace) => &workspace.selection,
        }
    }

    pub fn selection_mut(&mut self) -> &mut PersistableSelection {
        match self {
            Workspace::Board(workspace) => &mut workspace.selection,
            Workspace::Autorouter(workspace) => &mut workspace.selection,
        }
    }

    pub fn board(&self) -> &Board {
        match self {
            Workspace::Board(workspace) => &workspace.board,
            Workspace::Autorouter(workspace) => {
                workspace.autorouter.router().navmesher_board().board()
            }
        }
    }

    pub fn board_mut(&mut self) -> &mut Board {
        match self {
            Workspace::Board(workspace) => &mut workspace.board,
            Workspace::Autorouter(_workspace) => todo!(),
            /*Workspace::Autorouter(workspace) => {
                workspace.autorouter.router().navmesher_board().board()
            }*/
        }
    }
}

pub struct BoardWorkspace {
    pub board: Board,
    pub selection: PersistableSelection,
}

impl BoardWorkspace {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            selection: PersistableSelection::new(),
        }
    }
}

pub struct AutorouterWorkspace {
    pub autorouter: Autorouter,
    pub selection: PersistableSelection,
}

impl AutorouterWorkspace {
    pub fn new(board: Board) -> Self {
        Self {
            autorouter: Autorouter::new(board),
            selection: PersistableSelection::new(),
        }
    }
}
