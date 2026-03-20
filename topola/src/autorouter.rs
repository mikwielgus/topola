// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{Board, Ratsnest, navmesher::NavmesherBoard};

#[derive(Clone, Debug, Getters)]
pub struct Autorouter {
    navmesher_board: NavmesherBoard,
    ratsnest: Ratsnest,
}

impl Autorouter {
    pub fn new(board: Board) -> Self {
        let ratsnest = Ratsnest::new(&board);

        Self {
            navmesher_board: NavmesherBoard::new(board),
            ratsnest,
        }
    }
}
