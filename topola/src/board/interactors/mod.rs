// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod drag_selection;
mod selection;

pub use drag_selection::{DragSelectionInteractor, DragSelectionOptions};
pub use selection::SelectionInteractor;
use serde::{Deserialize, Serialize};

use crate::Vector2;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct InteractiveInput {
    pointer: Vector2<i64>,
    released: bool,
    cancel: bool,
}

impl InteractiveInput {
    pub fn new(pointer: Vector2<i64>, released: bool, cancel: bool) -> Self {
        Self {
            pointer,
            released,
            cancel,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum SelectionCombineMode {
    Replace,
    Additive,
    Subtractive,
    Toggle,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum SelectionContainMode {
    Crossing,
    Window,
}
