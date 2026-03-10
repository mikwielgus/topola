// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT

use bimap::BiBTreeMap;
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
    #[getter(skip)]
    layer_names: BiBTreeMap<usize, String>,
    #[getter(skip)]
    net_names: BiBTreeMap<usize, String>,
}

impl Board {
    pub fn new(boundary: Vec<[i64; 2]>, layer_count: usize) -> Self {
        Self {
            layout: Layout::new(boundary, layer_count),
            layer_names: BiBTreeMap::new(),
            net_names: BiBTreeMap::new(),
        }
    }

    pub fn with_names(
        boundary: Vec<[i64; 2]>,
        layer_count: usize,
        layer_names: BiBTreeMap<usize, String>,
        net_names: BiBTreeMap<usize, String>,
    ) -> Self {
        Self {
            layout: Layout::new(boundary, layer_count),
            layer_names,
            net_names,
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

    pub fn layer_name(&self, layer: usize) -> Option<&str> {
        self.layer_names.get_by_left(&layer).map(String::as_str)
    }

    pub fn layer_id(&self, name: &str) -> Option<usize> {
        self.layer_names.get_by_right(name).copied()
    }

    pub fn net_name(&self, net: usize) -> Option<&str> {
        self.net_names.get_by_left(&net).map(String::as_str)
    }

    pub fn net_id(&self, name: &str) -> Option<usize> {
        self.net_names.get_by_right(name).copied()
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
