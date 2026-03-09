// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use topola::Board;

use crate::translator::Translator;

pub struct Workspace {
    pub board: Board,
}

impl Workspace {
    pub fn new(board: Board, tr: &Translator) -> Self {
        Self { board }
    }
}
