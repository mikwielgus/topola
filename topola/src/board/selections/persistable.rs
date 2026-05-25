// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

use crate::{
    board::selections::{ComponentSelection, PinSelection},
    selections::route::RouteSelection,
};

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PersistableSelection {
    pub components: ComponentSelection,
    pub routes: RouteSelection,
    pub pins: PinSelection,
}

impl PersistableSelection {
    pub fn new() -> Self {
        Self {
            components: ComponentSelection::new(),
            routes: RouteSelection::new(),
            pins: PinSelection::new(),
        }
    }
}
