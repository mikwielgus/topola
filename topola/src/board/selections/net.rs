// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use derive_more::IntoIterator;
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

#[derive(
    Clone, Debug, Default, Deserialize, Eq, IntoIterator, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct NetSelection(pub BTreeSet<NetSelector>);

impl NetSelection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, selectors: impl IntoIterator<Item = NetSelector>) {
        self.0.extend(selectors);
    }

    pub fn sub(&mut self, selectors: impl IntoIterator<Item = NetSelector>) {
        for selector in selectors {
            self.0.remove(&selector);
        }
    }

    pub fn xor(&mut self, selectors: impl IntoIterator<Item = NetSelector>) {
        for selector in selectors {
            if self.0.contains(&selector) {
                self.0.remove(&selector);
            } else {
                self.0.insert(selector);
            }
        }
    }
}
