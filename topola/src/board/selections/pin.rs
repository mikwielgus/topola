// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Constructor, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinSelector {
    pub pin: String,
    pub layer: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PinSelection(pub BTreeSet<PinSelector>);

impl PinSelection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn toggle(&mut self, selector: PinSelector) {
        if self.0.contains(&selector) {
            self.0.remove(&selector);
        } else {
            self.0.insert(selector);
        }
    }
}
