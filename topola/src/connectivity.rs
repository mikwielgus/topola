// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use polygon_unionfind::UnionFind;

use crate::Board;

pub struct Connectivity {
    joints_unionfind: UnionFind,
    segments_unionfind: UnionFind,
    polygons_unionfind: UnionFind,
}

impl Connectivity {
    pub fn new(board: &Board) -> Self {
        let mut this = Connectivity {
            joints_unionfind: UnionFind::with_len(
                board.layout().joints().collection().num_elements(),
            ),
            segments_unionfind: UnionFind::with_len(
                board.layout().segments().collection().num_elements(),
            ),
            polygons_unionfind: UnionFind::with_len(
                board.layout().polygons().collection().num_elements(),
            ),
        };

        TODO
    }
}
