// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bimap::BiBTreeMap;
use derive_getters::{Dissolve, Getters};
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::{
    layout::{Layout, LayoutHalfDelta, NetId, PinId},
    math::Vector2,
    primitives::{
        Joint, JointId, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId, ViaSpec,
    },
    selection::{PinSelection, PinSelector},
};

#[derive(Clone, Debug, Getters)]
pub struct Board {
    layout: Layout,
    #[getter(skip)]
    pin_names: BiBTreeMap<PinId, String>,
    #[getter(skip)]
    layer_names: BiBTreeMap<usize, String>,
    #[getter(skip)]
    net_names: BiBTreeMap<NetId, String>,
}

impl Board {
    pub fn new(boundary: Vec<Vector2<i64>>, layer_count: usize) -> Self {
        Self {
            layout: Layout::new(boundary.into_iter().map(Into::into).collect(), layer_count),
            pin_names: BiBTreeMap::new(),
            layer_names: BiBTreeMap::new(),
            net_names: BiBTreeMap::new(),
        }
    }

    pub fn with_names(
        boundary: Vec<Vector2<i64>>,
        layer_count: usize,
        layer_names: BiBTreeMap<usize, String>,
        net_names: BiBTreeMap<NetId, String>,
    ) -> Self {
        Self {
            layout: Layout::new(boundary.into_iter().map(Into::into).collect(), layer_count),
            pin_names: BiBTreeMap::new(),
            layer_names,
            net_names,
        }
    }

    pub fn ensure_pin(&mut self, pin_name: String) -> PinId {
        if let Some(pin) = self.pin_names.get_by_right(&pin_name) {
            return *pin;
        };

        let pin_id = self.layout.add_pin();
        self.pin_names.insert(pin_id, pin_name);

        pin_id
    }

    pub fn add_joint(&mut self, joint: Joint) -> JointId {
        self.layout.add_joint(joint)
    }

    pub fn add_segment(&mut self, spec: SegmentSpec) -> SegmentId {
        self.layout.add_segment(spec)
    }

    pub fn add_segment_raw(&mut self, segment: Segment) -> SegmentId {
        self.layout.add_segment_raw(segment)
    }

    pub fn add_via(&mut self, spec: ViaSpec) -> ViaId {
        self.layout.add_via(spec)
    }

    pub fn add_via_raw(&mut self, via: Via) -> ViaId {
        self.layout.add_via_raw(via)
    }

    pub fn add_polygon(&mut self, polygon: Polygon) -> PolygonId {
        self.layout.add_polygon(polygon)
    }

    pub fn joint_pin_selector(&self, joint_id: JointId) -> Option<PinSelector> {
        let joint = self.layout.joint(joint_id);

        Some(PinSelector {
            pin: self.pin_name(joint.pin?)?.to_string(),
            layer: self.layer_name(joint.layer)?.to_string(),
        })
    }

    pub fn segment_pin_selector(&self, segment_id: SegmentId) -> Option<PinSelector> {
        let segment = self.layout.segment(segment_id);

        Some(PinSelector {
            pin: self.pin_name(segment.spec.pin?)?.to_string(),
            layer: self.layer_name(segment.layer)?.to_string(),
        })
    }

    // TODO: Vias.

    pub fn polygon_pin_selector(&self, polygon_id: PolygonId) -> Option<PinSelector> {
        let polygon = self.layout.polygon(polygon_id);

        Some(PinSelector {
            pin: self.pin_name(polygon.pin?)?.to_string(),
            layer: self.layer_name(polygon.layer)?.to_string(),
        })
    }

    pub fn point_pin_selector(&self, layer: usize, point: Vector2<i64>) -> Option<PinSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_pin_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_pin_selector(segment_id);
        }

        // TODO: Vias.

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_pin_selector(polygon_id);
        }

        None
    }

    pub fn pin_selection_contains_joint(
        &self,
        pin_selection: &PinSelection,
        joint_id: JointId,
    ) -> bool {
        let Some(pin_selector) = self.joint_pin_selector(joint_id) else {
            return false;
        };

        pin_selection.0.contains(&pin_selector)
    }

    pub fn pin_selection_contains_segment(
        &self,
        pin_selection: &PinSelection,
        segment_id: SegmentId,
    ) -> bool {
        let Some(pin_selector) = self.segment_pin_selector(segment_id) else {
            return false;
        };

        pin_selection.0.contains(&pin_selector)
    }

    // TODO: Vias.

    pub fn pin_selection_contains_polygon(
        &self,
        pin_selection: &PinSelection,
        polygon_id: PolygonId,
    ) -> bool {
        let Some(pin_selector) = self.polygon_pin_selector(polygon_id) else {
            return false;
        };

        pin_selection.0.contains(&pin_selector)
    }

    pub fn pin_name(&self, pin: PinId) -> Option<&str> {
        self.pin_names.get_by_left(&pin).map(String::as_str)
    }

    pub fn pin_id(&self, pin_name: &str) -> Option<PinId> {
        self.pin_names.get_by_right(pin_name).copied()
    }

    pub fn layer_name(&self, layer: usize) -> Option<&str> {
        self.layer_names.get_by_left(&layer).map(String::as_str)
    }

    pub fn layer_id(&self, layer_name: &str) -> Option<usize> {
        self.layer_names.get_by_right(layer_name).copied()
    }

    pub fn net_name(&self, net: NetId) -> Option<&str> {
        self.net_names.get_by_left(&net).map(String::as_str)
    }

    pub fn net_id(&self, net_name: &str) -> Option<NetId> {
        self.net_names.get_by_right(net_name).copied()
    }
}

#[derive(Clone, Debug, Dissolve)]
pub struct BoardHalfDelta {
    layout: LayoutHalfDelta,
}

impl ApplyDelta<BoardHalfDelta> for Board {
    fn apply_delta(&mut self, delta: Delta<BoardHalfDelta>) {
        let (removed, inserted) = delta.dissolve();

        let layout_delta = Delta::with_removed_inserted(removed.layout, inserted.layout);
        self.layout.apply_delta(layout_delta);
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
