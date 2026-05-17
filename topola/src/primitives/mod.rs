// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

mod joint;
mod polygon;
mod segment;
mod via;

pub use joint::{Joint, JointId};
pub use polygon::{Polygon, PolygonId};
pub use segment::{Segment, SegmentId, SegmentSpec};
pub use via::{Via, ViaId, ViaSpec};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PrimitiveId {
    Joint(JointId),
    Segment(SegmentId),
    Polygon(PolygonId),
}
