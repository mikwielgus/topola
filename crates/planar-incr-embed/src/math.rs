// SPDX-FileCopyrightText: 2025 Topola contributors
// SPDX-FileCopyrightText: 2024 Spade contributors
// SPDX-FileCopyrightText: 2024 Stefan Altmayer <stoeoef@gmail.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use num_traits::float::Float;
use spade::{Point2, SpadeNum};

// source: https://github.com/Stoeoef/spade/blob/083b7744d91b994f4b482cdfaf0787a049e94858/src/delaunay_core/math.rs,
// starting at line 359, modified to avoid private mothods of spade
pub fn circumcenter<S>(positions: [Point2<S>; 3]) -> (Point2<S>, S)
where
    S: SpadeNum + Float,
{
    let [v0, v1, v2] = positions;
    let b = Point2 {
        x: v1.x - v0.x,
        y: v1.y - v0.y,
    };
    let c = Point2 {
        x: v2.x - v0.x,
        y: v2.y - v0.y,
    };

    let one = S::one();
    let two = one + one;
    let d = two * (b.x * c.y - c.x * b.y);
    let len_b = b.x * b.x + b.y * b.y;
    let len_c = c.x * c.x + c.y * c.y;
    let d_inv: S = one / d;

    let x = (len_b * c.y - len_c * b.y) * d_inv;
    let y = (-len_b * c.x + len_c * b.x) * d_inv;
    let result = Point2::new(x + v0.x, y + v0.y);
    (result, x * x + y * y)
}
