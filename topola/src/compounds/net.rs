// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Constructor, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct NetId(usize);

impl NetId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}
