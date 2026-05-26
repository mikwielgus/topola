// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

use crate::board::selections::{ComponentSelection, NetSelection, PinSelection};

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PersistableSelection {
    pub components: ComponentSelection,
    pub nets: NetSelection,
    pub pins: PinSelection,
}

impl PersistableSelection {
    pub fn new() -> Self {
        Self {
            components: ComponentSelection::new(),
            nets: NetSelection::new(),
            pins: PinSelection::new(),
        }
    }
}
