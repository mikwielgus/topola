// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ComponentSelector {
    pub component: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentSelection(pub BTreeSet<ComponentSelector>);

impl ComponentSelection {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn toggle(&mut self, selector: ComponentSelector) {
        if self.0.contains(&selector) {
            self.0.remove(&selector);
        } else {
            self.0.insert(selector);
        }
    }
}
