// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use polygon_unionfind::UnionFind;

use crate::Vector2;

/// Returns the four vertices of a seg inflated by `half_width`, forming a
/// convex quadrilateral. The seg goes from (x1, y1) to (x2, y2).
pub fn inflated_seg(x1: i64, y1: i64, x2: i64, y2: i64, half_width: u64) -> [Vector2<i64>; 4] {
    let dx = x2 - x1;
    let dy = y2 - y1;

    let approx_len = std::cmp::max(dx.abs(), dy.abs()) + 3 * std::cmp::min(dx.abs(), dy.abs()) / 8;

    // Perpendicular vector scaled to half-width.
    let px = -dy * (half_width as i64) / approx_len;
    let py = dx * (half_width as i64) / approx_len;

    [
        Vector2::new(x1 + px, y1 + py),
        Vector2::new(x2 + px, y2 + py),
        Vector2::new(x2 - px, y2 - py),
        Vector2::new(x1 - px, y1 - py),
    ]
}

/// Kruskal's minimum spanning tree algorithm.
pub fn kruskal_mst<W: Copy + Ord>(
    vertex_count: usize,
    edges: &[(W, [usize; 2])],
) -> Vec<[usize; 2]> {
    let mut sorted_edges = edges.to_vec();
    sorted_edges.sort_by_key(|(w, _)| *w);

    let mut unionfind: UnionFind = UnionFind::with_len(vertex_count);
    let mut min_spanning_tree = Vec::new();

    for (_, uv) in sorted_edges {
        if unionfind.union(uv[0], uv[1]) {
            min_spanning_tree.push(uv);
        }
    }

    min_spanning_tree
}

#[cfg(test)]
mod tests {
    use super::kruskal_mst;

    #[test]
    fn kruskal_mst_on_path_graph() {
        // Path graph. Graph edges just form a sequence without cycle.
        let edges = [(1, [0, 1]), (2, [1, 2]), (3, [2, 3])];
        let mst = kruskal_mst(4, &edges);

        assert_eq!(mst.len(), 3);
        assert_eq!(mst, [[0, 1], [1, 2], [2, 3]]);
    }

    #[test]
    fn kruskal_mst_on_triangle() {
        // Triangle graph. Graph edges form a triangle.
        let edges = [(1, [0, 1]), (2, [1, 2]), (3, [0, 2])];
        let mst = kruskal_mst(3, &edges);

        assert_eq!(mst.len(), 2);
        assert!(mst.contains(&[0, 1]));
        assert!(mst.contains(&[1, 2]));
    }
}
