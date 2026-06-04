// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_getters::Getters;

use crate::{board::Board, ratsnest::Ratsnest, router::Router};

#[derive(Clone, Debug, Getters)]
pub struct Autorouter {
    ratsnest: Ratsnest,
    router: Router,
}

impl Autorouter {
    pub fn new(board: Board) -> Self {
        let ratsnest = Ratsnest::new(&board);

        Self {
            router: Router::new(board),
            ratsnest,
        }
    }
}
