// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use bimap::BiBTreeMap;
use derive_getters::{Dissolve, Getters};
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::{
    compounds::{ComponentId, NetId, PinId},
    layout::{Layout, LayoutHalfDelta},
    math::Vector2,
    primitives::{
        JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
        ViaSpec,
    },
    selections::{
        ComponentSelection, ComponentSelector, PinWithLayerSelection, PinWithLayerSelector,
    },
};

#[derive(Clone, Debug, Getters)]
pub struct Board {
    layout: Layout,
    #[getter(skip)]
    component_names: BiBTreeMap<ComponentId, String>,
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
            component_names: BiBTreeMap::new(),
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
            component_names: BiBTreeMap::new(),
            pin_names: BiBTreeMap::new(),
            layer_names,
            net_names,
        }
    }

    pub fn ensure_named_component(&mut self, component_name: String) -> ComponentId {
        if let Some(component) = self.component_names.get_by_right(&component_name) {
            return *component;
        };

        let component_id = self.layout.add_component();
        self.component_names.insert(component_id, component_name);

        component_id
    }

    pub fn ensure_named_pin(&mut self, pin_name: String) -> PinId {
        if let Some(pin) = self.pin_names.get_by_right(&pin_name) {
            return *pin;
        };

        let pin_id = self.layout.add_pin();
        self.pin_names.insert(pin_id, pin_name);

        pin_id
    }

    pub fn add_component(&mut self) -> ComponentId {
        self.layout.add_component()
    }

    pub fn add_joint(&mut self, spec: JointSpec) -> JointId {
        self.layout.add_joint(spec)
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

    pub fn joint_component_selector(&self, id: JointId) -> Option<ComponentSelector> {
        let joint = self.layout.joint(id);

        Some(ComponentSelector {
            component: self.component_name(joint.spec.component?)?.to_string(),
        })
    }

    pub fn segment_component_selector(&self, id: SegmentId) -> Option<ComponentSelector> {
        let segment = self.layout.segment(id);

        Some(ComponentSelector {
            component: self.component_name(segment.spec.component?)?.to_string(),
        })
    }

    // TODO: Vias.

    pub fn polygon_component_selector(&self, id: PolygonId) -> Option<ComponentSelector> {
        let polygon = self.layout.polygon(id);

        Some(ComponentSelector {
            component: self.component_name(polygon.component?)?.to_string(),
        })
    }

    pub fn point_component_selector(
        &self,
        layer: usize,
        point: Vector2<i64>,
    ) -> Option<ComponentSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_component_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_component_selector(segment_id);
        }

        // TODO: Vias.

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_component_selector(polygon_id);
        }

        None
    }

    pub fn component_selection_contains_joint(
        &self,
        selection: &ComponentSelection,
        id: JointId,
    ) -> bool {
        let Some(selector) = self.joint_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn component_selection_contains_segment(
        &self,
        selection: &ComponentSelection,
        id: SegmentId,
    ) -> bool {
        let Some(selector) = self.segment_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    // TODO: Vias.

    pub fn component_selection_contains_polygon(
        &self,
        selection: &ComponentSelection,
        id: PolygonId,
    ) -> bool {
        let Some(selector) = self.polygon_component_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn joint_pin_with_layer_selector(&self, id: JointId) -> Option<PinWithLayerSelector> {
        let joint = self.layout.joint(id);

        Some(PinWithLayerSelector {
            pin: self.pin_name(joint.spec.pin?)?.to_string(),
            layer: self.layer_name(joint.spec.layer)?.to_string(),
        })
    }

    pub fn segment_pin_with_layer_selector(&self, id: SegmentId) -> Option<PinWithLayerSelector> {
        let segment = self.layout.segment(id);

        Some(PinWithLayerSelector {
            pin: self.pin_name(segment.spec.pin?)?.to_string(),
            layer: self.layer_name(segment.layer)?.to_string(),
        })
    }

    // TODO: Vias.

    pub fn polygon_pin_with_layer_selector(&self, id: PolygonId) -> Option<PinWithLayerSelector> {
        let polygon = self.layout.polygon(id);

        Some(PinWithLayerSelector {
            pin: self.pin_name(polygon.pin?)?.to_string(),
            layer: self.layer_name(polygon.layer)?.to_string(),
        })
    }

    pub fn point_pin_with_layer_selector(
        &self,
        layer: usize,
        point: Vector2<i64>,
    ) -> Option<PinWithLayerSelector> {
        if let Some(joint_id) = self.layout.locate_joints_at_point(layer, point).next() {
            return self.joint_pin_with_layer_selector(joint_id);
        }

        if let Some(segment_id) = self.layout.locate_segments_at_point(layer, point).next() {
            return self.segment_pin_with_layer_selector(segment_id);
        }

        // TODO: Vias.

        if let Some(polygon_id) = self.layout.locate_polygons_at_point(layer, point).next() {
            return self.polygon_pin_with_layer_selector(polygon_id);
        }

        None
    }

    pub fn pin_with_layer_selection_contains_joint(
        &self,
        selection: &PinWithLayerSelection,
        id: JointId,
    ) -> bool {
        let Some(selector) = self.joint_pin_with_layer_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn pin_with_layer_selection_contains_segment(
        &self,
        selection: &PinWithLayerSelection,
        id: SegmentId,
    ) -> bool {
        let Some(selector) = self.segment_pin_with_layer_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    // TODO: Vias.

    pub fn pin_with_layer_selection_contains_polygon(
        &self,
        selection: &PinWithLayerSelection,
        id: PolygonId,
    ) -> bool {
        let Some(selector) = self.polygon_pin_with_layer_selector(id) else {
            return false;
        };

        selection.0.contains(&selector)
    }

    pub fn component_name(&self, id: ComponentId) -> Option<&str> {
        self.component_names.get_by_left(&id).map(String::as_str)
    }

    pub fn component_id(&self, component_name: &str) -> Option<ComponentId> {
        self.component_names.get_by_right(component_name).copied()
    }

    pub fn pin_name(&self, id: PinId) -> Option<&str> {
        self.pin_names.get_by_left(&id).map(String::as_str)
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

    pub fn net_name(&self, id: NetId) -> Option<&str> {
        self.net_names.get_by_left(&id).map(String::as_str)
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
