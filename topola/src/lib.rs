// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod profiler;

mod autoplacer;
pub mod board;
mod compass;
mod interactor;
pub mod layout;
mod math;
mod orientation;
mod ratsnest;
mod rect;
mod specctra;
mod vector;
mod workspace;

pub use crate::autoplacer::AutoplacerSchedule;
pub use crate::board::BoardDelta;
pub use crate::board::selections;
pub use crate::interactor::{Interactor, MasterInteractor};
pub use crate::orientation::Orientation;
pub use crate::ratsnest::{Ratline, Ratsnest};
pub use crate::rect::{Rect2, Rect3};
pub use crate::vector::{Vector2, Vector3};
pub use crate::workspace::Workspace;
