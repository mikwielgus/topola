// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod autorouter;
mod board;
mod compass;
mod drawer;
mod layout;
mod math;
mod navmesher;
mod pathfinder;
mod ratsnest;
mod rect;
mod router;
mod specctra;
mod vector;
mod workspace;

pub use crate::autorouter::Autorouter;
pub use crate::board::Board;
pub use crate::board::LayerDesc;
pub use crate::board::LayerSide;
pub use crate::board::LayerType;
pub use crate::board::interactors::{
    DragSelectInteractor, DragSelectOptions, MasterInteractor, SelectInteractor,
    SelectionCombineMode, SelectionContainMode,
};
pub use crate::board::selections;
pub use crate::layout::LayerId;
pub use crate::layout::Layout;
pub use crate::layout::compounds::{Pin, PinId};
pub use crate::layout::primitives;
pub use crate::ratsnest::{Ratline, Ratsnest};
pub use crate::rect::{Rect2, Rect3};
pub use crate::vector::{Vector2, Vector3};
pub use crate::workspace::{AutorouterWorkspace, BoardWorkspace, Workspace};
