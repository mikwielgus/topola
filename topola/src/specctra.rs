// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use specctra::structure::DsnFile;

use crate::board::Board;

impl Board {
    pub fn from_specctra(dsn: DsnFile) -> Self {
        Board::new(
            dsn.pcb
                .structure
                .boundary
                .coords()
                .into_owned()
                .into_iter()
                .map(|p| [p.x as i64, p.y as i64])
                .collect(),
        )
    }
}
