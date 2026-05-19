// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod select;
pub mod selections;

use bimap::BiBTreeMap;
use derive_getters::{Dissolve, Getters};
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::{
    compounds::{ComponentId, NetId, PinId},
    layout::{
        Layout, LayoutHalfDelta,
        primitives::{
            JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
            ViaSpec,
        },
    },
    math::Vector2,
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
