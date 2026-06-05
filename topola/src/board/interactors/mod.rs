// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod drag_move;
mod drag_select;
mod master;
mod select;

pub use drag_move::DragMoveInteractor;
pub use drag_select::{DragSelectInteractor, DragSelectOptions};
pub use master::BoardMasterInteractor;
pub use select::SelectInteractor;
use serde::{Deserialize, Serialize};

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
