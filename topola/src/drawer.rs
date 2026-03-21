// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::navmesher::NavmesherBoard;

#[derive(Clone, Debug, Getters)]
pub struct Drawer {
    navmesher_board: NavmesherBoard,
}

impl Drawer {
    pub fn new(navmesher_board: NavmesherBoard) -> Self {
        Self { navmesher_board }
    }
}
