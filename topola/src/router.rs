// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{board::Board, drawer::Drawer, navmesher::NavmesherBoard, pathfinder::Pathfinder};

#[derive(Clone, Debug)]
pub enum Router {
    Resting(NavmesherBoard),
    Pathfinder(Pathfinder),
    Drawer(Drawer),
}

impl Router {
    pub fn new(board: Board) -> Self {
        Self::Resting(NavmesherBoard::new(board))
    }

    pub fn navmesher_board(&self) -> &NavmesherBoard {
        match self {
            Router::Resting(navmesher_board) => navmesher_board,
            Router::Pathfinder(pathfinder) => pathfinder.navmesher_board(),
            Router::Drawer(drawer) => drawer.navmesher_board(),
        }
    }
}
