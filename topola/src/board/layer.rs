// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt::Display;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LayerType {
    Copper,
    Outline,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LayerSide {
    Top,
    Inner,
    Bottom,
}

#[derive(Clone, Constructor, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct LayerDesc {
    pub typ: LayerType,
    pub side: LayerSide,
    pub index: usize,
}

impl Display for LayerDesc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.typ {
            LayerType::Copper => match self.side {
                LayerSide::Top => write!(f, "F.Cu"),
                LayerSide::Bottom => write!(f, "B.Cu"),
                LayerSide::Inner => write!(f, "In{}.Cu", self.index.saturating_sub(1)),
            },
            LayerType::Outline => match self.side {
                LayerSide::Top => write!(f, "F.Outl"),
                LayerSide::Bottom => write!(f, "B.Outl"),
                LayerSide::Inner => write!(f, "In{}.Outl", self.index),
            },
        }
    }
}
