// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use poly_unionfind::UnionFind;

use crate::board::Board;

pub struct Connectivity {
    joints_unionfind: UnionFind,
    segs_unionfind: UnionFind,
    polys_unionfind: UnionFind,
}

impl Connectivity {
    pub fn new(board: &Board) -> Self {
        let mut this = Connectivity {
            joints_unionfind: UnionFind::with_len(
                board.layout().joints().container().num_elements(),
            ),
            segs_unionfind: UnionFind::with_len(
                board.layout().segs().container().num_elements(),
            ),
            polys_unionfind: UnionFind::with_len(
                board.layout().polys().container().num_elements(),
            ),
        };

        TODO
    }
}
