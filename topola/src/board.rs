// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::{Dissolve, Getters};
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::layout::{
    Arc, ArcId, Joint, JointId, Layout, LayoutHalfDelta, Polygon, PolygonId, Segment, SegmentId,
    Via, ViaId,
};

struct Layer {
    name: String,
    index: usize,
}

#[derive(Clone, Debug, Getters)]
pub struct Board {
    layout: Layout,
}

impl Board {
    pub fn new(boundary: Vec<[i64; 2]>) -> Self {
        Self {
            layout: Layout::new(boundary),
        }
    }

    pub fn add_joint(&mut self, joint: Joint) -> JointId {
        self.layout.add_joint(joint)
    }

    pub fn add_segment(&mut self, segment: Segment) -> SegmentId {
        self.layout.add_segment(segment)
    }

    pub fn add_arc(&mut self, arc: Arc) -> ArcId {
        self.layout.add_arc(arc)
    }

    pub fn add_via(&mut self, via: Via) -> ViaId {
        self.layout.add_via(via)
    }

    pub fn add_polygon(&mut self, polygon: Polygon) -> PolygonId {
        self.layout.add_polygon(polygon)
    }
}

#[derive(Clone, Debug, Dissolve)]
pub struct BoardHalfDelta {
    layout: LayoutHalfDelta,
}

impl ApplyDelta<BoardHalfDelta> for Board {
    fn apply_delta(&mut self, delta: &Delta<BoardHalfDelta>) {
        let (removed, inserted) = delta.clone().dissolve();

        let layout_delta = Delta::with_removed_inserted(removed.layout, inserted.layout);
        self.layout.apply_delta(&layout_delta);
    }
}

impl FlushDelta<BoardHalfDelta> for Board {
    fn flush_delta(&mut self) -> Delta<BoardHalfDelta> {
        let (removed_layout, inserted_layout) = self.layout.flush_delta().dissolve();

        Delta::with_removed_inserted(
            BoardHalfDelta {
                layout: removed_layout,
            },
            BoardHalfDelta {
                layout: inserted_layout,
            },
        )
    }
}
