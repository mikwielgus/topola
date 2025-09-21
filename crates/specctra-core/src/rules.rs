// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::borrow::Cow;

pub trait GetConditions<'a> {
    fn conditions(self) -> Option<Conditions<'a>>;
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Conditions<'a> {
    pub net: usize,

    #[serde(borrow)]
    pub maybe_region: Option<Cow<'a, str>>,

    pub maybe_layer: Option<usize>,
}

pub trait AccessRules {
    fn clearance(&self, conditions1: &Conditions<'_>, conditions2: &Conditions<'_>) -> Option<f64>;
    fn largest_clearance(&self, net: Option<usize>) -> f64;
}
