// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{board::Board, router::Router};

#[derive(Clone, Debug, Getters)]
pub struct Autorouter {
    router: Router,
}

impl Autorouter {
    pub fn new(board: Board) -> Self {
        Self {
            router: Router::new(board),
        }
    }
}
