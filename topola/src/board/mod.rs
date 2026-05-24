// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

mod layer;
mod resolve;
mod select;
pub mod selections;
mod transforms;

pub use crate::board::layer::{LayerDesc, LayerGroupId, LayerTier, LayerType};

use bidimap::BiBTreeMap;
use derive_getters::Getters;
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
    layer_descs: Recorder<BiBTreeMap<LayerId, LayerDesc>>,
    #[getter(skip)]
    net_names: Recorder<BiBTreeMap<NetId, String>>,
}

impl Board {
    /*pub fn new(boundary: Vec<Vector2<i64>>, layer_count: usize) -> Self {
        Self {
            layout: Layout::new(boundary.into_iter().map(Into::into).collect(), layer_count),
            component_names: Recorder::new(BiBTreeMap::new()),
            pin_names: Recorder::new(BiBTreeMap::new()),
            layer_descs: Recorder::new(BiBTreeMap::new()),
            net_names: Recorder::new(BiBTreeMap::new()),
        }
    }*/

    pub fn with_names(
        boundary: Vec<Vector2<i64>>,
        layer_groups: Vec<LayerGroupId>,
        layer_descs: BiBTreeMap<LayerId, LayerDesc>,
        net_names: BiBTreeMap<NetId, String>,
    ) -> Self {
        Self {
            layout: Layout::new(
                boundary.into_iter().map(Into::into).collect(),
                layer_groups.len(),
            ),
            layer_groups: Recorder::new(layer_groups),
            component_names: Recorder::new(BiBTreeMap::new()),
            pin_names: Recorder::new(BiBTreeMap::new()),
            layer_descs: Recorder::new(layer_descs),
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

    pub fn layer_name(&self, layer: LayerId) -> Option<String> {
        self.layer_descs
            .get_by_left(&layer)
            .map(ToString::to_string)
    }

    pub fn layer_desc(&self, layer: LayerId) -> Option<&LayerDesc> {
        self.layer_descs.get_by_left(&layer)
    }

    pub fn layer_id(&self, layer_name: &str) -> Option<LayerId> {
        self.layer_descs
            .as_ref()
            .iter()
            .find_map(|(layer_id, layer_desc)| {
                (layer_desc.to_string() == layer_name).then_some(*layer_id)
            })
    }

    pub fn layer_group(&self, layer: LayerId) -> LayerGroupId {
        self.layer_groups[layer.index()]
    }

    pub fn net_name(&self, id: NetId) -> Option<&str> {
        self.net_names.get_by_left(&id).map(String::as_str)
    }

    pub fn net_id(&self, net_name: &str) -> Option<NetId> {
        self.net_names.as_ref().get_by_right(net_name).copied()
    }
}
