// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use crate::layout::{NetId, PinId};

use super::joint::JointId;

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct ViaId(usize);

impl ViaId {
    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Via {
    pub endjoints: [JointId; 2],
    pub layer: usize, // ??? This should be a range.
    pub radius: u64,
    pub net: NetId,
    pub pin: Option<PinId>,
}

impl Via {
    /*pub fn bbox(&self) -> Rectangle<[i64; 3]> {
        //
    }*/
}
