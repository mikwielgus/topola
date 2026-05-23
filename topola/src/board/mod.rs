// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod resolve;
mod select;
pub mod selections;
mod transforms;

use bidimap::BiBTreeMap;
use derive_getters::Getters;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use undoredo::{Delta, Recorder};

use crate::{
    layout::{
        LayerId, Layout, LayoutHalfDelta,
        compounds::{ComponentId, NetId, PinId},
        primitives::{
            JointId, JointSpec, Polygon, PolygonId, Segment, SegmentId, SegmentSpec, Via, ViaId,
            ViaSpec,
        },
    },
    math::Vector2,
};

#[derive(
    Clone,
    Constructor,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    From,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct LayerGroupId(usize);

impl LayerGroupId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Getters, Delta)]
pub struct Board {
    layout: Layout,
    #[getter(skip)]
    layer_groups: Recorder<Vec<LayerGroupId>>,
    #[getter(skip)]
    component_names: Recorder<BiBTreeMap<ComponentId, String>>,
    #[getter(skip)]
    pin_names: Recorder<BiBTreeMap<PinId, String>>,
    #[getter(skip)]
    layer_names: Recorder<BiBTreeMap<LayerId, String>>,
    #[getter(skip)]
    net_names: Recorder<BiBTreeMap<NetId, String>>,
}

impl Board {
    /*pub fn new(boundary: Vec<Vector2<i64>>, layer_count: usize) -> Self {
        Self {
            layout: Layout::new(boundary.into_iter().map(Into::into).collect(), layer_count),
            component_names: Recorder::new(BiBTreeMap::new()),
            pin_names: Recorder::new(BiBTreeMap::new()),
            layer_names: Recorder::new(BiBTreeMap::new()),
            net_names: Recorder::new(BiBTreeMap::new()),
        }
    }*/

    pub fn with_names(
        boundary: Vec<Vector2<i64>>,
        layer_groups: impl Into<Vec<LayerGroupId>>,
        layer_names: BiBTreeMap<LayerId, String>,
        net_names: BiBTreeMap<NetId, String>,
    ) -> Self {
        let layer_groups = layer_groups.into();
        Self {
            layout: Layout::new(
                boundary.into_iter().map(Into::into).collect(),
                layer_groups.len(),
            ),
            layer_groups: Recorder::new(layer_groups),
            component_names: Recorder::new(BiBTreeMap::new()),
            pin_names: Recorder::new(BiBTreeMap::new()),
            layer_names: Recorder::new(layer_names),
            net_names: Recorder::new(net_names),
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
        self.component_names
            .as_ref()
            .get_by_right(component_name)
            .copied()
    }

    pub fn pin_name(&self, id: PinId) -> Option<&str> {
        self.pin_names.get_by_left(&id).map(String::as_str)
    }

    pub fn pin_id(&self, pin_name: &str) -> Option<PinId> {
        self.pin_names.as_ref().get_by_right(pin_name).copied()
    }

    pub fn layer_name(&self, layer: LayerId) -> Option<&str> {
        self.layer_names.get_by_left(&layer).map(String::as_str)
    }

    pub fn layer_id(&self, layer_name: &str) -> Option<LayerId> {
        self.layer_names.as_ref().get_by_right(layer_name).copied()
    }

    pub fn net_name(&self, id: NetId) -> Option<&str> {
        self.net_names.get_by_left(&id).map(String::as_str)
    }

    pub fn net_id(&self, net_name: &str) -> Option<NetId> {
        self.net_names.as_ref().get_by_right(net_name).copied()
    }
}
