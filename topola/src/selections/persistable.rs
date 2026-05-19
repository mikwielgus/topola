// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

use crate::selections::{ComponentSelection, PinWithLayerSelection};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistableSelection {
    pub components: ComponentSelection,
    pub pins: PinWithLayerSelection,
}

impl PersistableSelection {
    pub fn new() -> Self {
        Self {
            components: ComponentSelection::new(),
            pins: PinWithLayerSelection::new(),
        }
    }
}
