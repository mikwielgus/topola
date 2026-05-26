// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod drag_selection;

pub use drag_selection::CrossingDragSelectionInteractor;

use crate::Vector2;

pub struct InteractiveInput {
    pointer: Vector2<i64>,
}

impl InteractiveInput {
    pub fn new(pointer: Vector2<i64>) -> Self {
        Self { pointer }
    }
}
