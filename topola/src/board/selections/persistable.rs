// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

use crate::board::selections::{ComponentSelection, PinSelection};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistableSelection {
    pub components: ComponentSelection,
    pub pins: PinSelection,
}

impl PersistableSelection {
    pub fn new() -> Self {
        Self {
            components: ComponentSelection::new(),
            pins: PinSelection::new(),
        }
    }
}
