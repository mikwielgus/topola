// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod drag_selection;
mod master;
mod selection;

use derive_more::Constructor;
pub use drag_selection::{DragSelectionInteractor, DragSelectionOptions};
pub use master::MasterInteractor;
pub use selection::SelectionInteractor;
use serde::{Deserialize, Serialize};

use crate::Vector2;

#[derive(Clone, Constructor, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct InteractiveInput {
    pointer: Vector2<i64>,
    release: bool,
    delete: bool,
    cancel: bool,
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
