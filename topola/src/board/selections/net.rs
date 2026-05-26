// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NetSelector {
    pub net: String,
}

impl NetSelector {
    pub fn new(net: String) -> Self {
        Self { net }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct NetSelection(pub BTreeSet<NetSelector>);

impl NetSelection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn toggle(&mut self, selector: NetSelector) {
        if self.0.contains(&selector) {
            self.0.remove(&selector);
        } else {
            self.0.insert(selector);
        }
    }
}
