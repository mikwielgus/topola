// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PinSelector {
    pub pin: String,
    pub layer: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PinSelection(pub BTreeSet<PinSelector>);

impl PinSelection {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn toggle(&mut self, selector: PinSelector) {
        if self.0.contains(&selector) {
            self.0.remove(&selector);
        } else {
            self.0.insert(selector);
        }
    }
}
