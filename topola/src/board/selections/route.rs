// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::selections::PinSelector;

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RouteSelector {
    lesser_pin: PinSelector,
    greater_pin: PinSelector,
}

impl RouteSelector {
    pub fn new(pin1: PinSelector, pin2: PinSelector) -> Self {
        Self {
            lesser_pin: std::cmp::min(pin1.clone(), pin2.clone()),
            greater_pin: std::cmp::max(pin1, pin2),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RouteSelection(pub BTreeSet<RouteSelector>);

impl RouteSelection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn toggle(&mut self, selector: RouteSelector) {
        if self.0.contains(&selector) {
            self.0.remove(&selector);
        } else {
            self.0.insert(selector);
        }
    }
}
