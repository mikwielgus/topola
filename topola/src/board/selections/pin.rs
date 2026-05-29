// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use derive_more::{Constructor, IntoIterator};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Constructor, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinSelector {
    pub pin: String,
    pub layer: String,
}

#[derive(
    Clone, Debug, Default, Deserialize, Eq, IntoIterator, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PinSelection(pub BTreeSet<PinSelector>);

impl PinSelection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, selectors: impl IntoIterator<Item = PinSelector>) {
        self.0.extend(selectors);
    }

    pub fn sub(&mut self, selectors: impl IntoIterator<Item = PinSelector>) {
        for selector in selectors {
            self.0.remove(&selector);
        }
    }

    pub fn toggle(&mut self, selectors: impl IntoIterator<Item = PinSelector>) {
        for selector in selectors {
            if self.0.contains(&selector) {
                self.0.remove(&selector);
            } else {
                self.0.insert(selector);
            }
        }
    }
}
