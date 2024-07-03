use std::collections::HashMap;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::{
        band::BandFirstSegIndex,
        dot::{FixedDotIndex, FixedDotWeight},
        graph::{GetLayer, GetMaybeNet},
        seg::{FixedSegIndex, FixedSegWeight},
    },
    geometry::{shape::AccessShape, GenericNode},
    graph::GenericIndex,
    layout::{
        zone::{GetMaybeApex, MakePolyShape, ZoneWeight},
        Layout, NodeIndex,
    },
    math::Circle,
    router::{navmesh::Navmesh, route::Route, Router, RouterError},
};

#[derive(Debug)]
pub struct Board<M: AccessMesadata> {
    layout: Layout<M>,
    node_to_pinname: HashMap<NodeIndex, String>,
    pinname_pair_to_band: HashMap<(String, String), BandFirstSegIndex>,
}

impl<M: AccessMesadata> Board<M> {
    pub fn new(layout: Layout<M>) -> Self {
        Self {
            layout,
            node_to_pinname: HashMap::new(),
            pinname_pair_to_band: HashMap::new(),
        }
    }

    pub fn add_fixed_dot_infringably(
        &mut self,
        weight: FixedDotWeight,
        maybe_pin: Option<String>,
    ) -> FixedDotIndex {
        let dot = self.layout.add_fixed_dot_infringably(weight);

        if let Some(ref pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(dot.into()), pin.clone());
        }

        dot
    }

    pub fn add_zone_fixed_dot_infringably(
        &mut self,
        weight: FixedDotWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> FixedDotIndex {
        let dot = self.layout.add_zone_fixed_dot_infringably(weight, zone);

        if let Some(pin) = self.node_pinname(GenericNode::Compound(zone.into())) {
            self.node_to_pinname
                .insert(GenericNode::Primitive(dot.into()), pin.to_string());
        }

        dot
    }

    pub fn add_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        maybe_pin: Option<String>,
    ) -> FixedSegIndex {
        let seg = self.layout.add_fixed_seg_infringably(from, to, weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin.to_string());
        }

        seg
    }

    pub fn add_zone_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> FixedSegIndex {
        let seg = self
            .layout
            .add_zone_fixed_seg_infringably(from, to, weight, zone);

        if let Some(pin) = self.node_pinname(GenericNode::Compound(zone.into())) {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin.to_string());
        }

        seg
    }

    pub fn add_zone(
        &mut self,
        weight: ZoneWeight,
        maybe_pin: Option<String>,
    ) -> GenericIndex<ZoneWeight> {
        let zone = self.layout.add_zone(weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Compound(zone.into()), pin.to_string());
        }

        zone
    }

    pub fn zone_apex(&mut self, zone: GenericIndex<ZoneWeight>) -> FixedDotIndex {
        if let Some(apex) = self.layout.zone(zone).maybe_apex() {
            apex
        } else {
            self.add_zone_fixed_dot_infringably(
                FixedDotWeight {
                    circle: Circle {
                        pos: self.layout.zone(zone).shape().center(),
                        r: 100.0,
                    },
                    layer: self.layout.zone(zone).layer(),
                    maybe_net: self.layout.zone(zone).maybe_net(),
                },
                zone,
            )
        }
    }

    pub fn node_pinname(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_pinname.get(&node)
    }

    pub fn try_set_band_between_nodes(
        &mut self,
        source: FixedDotIndex,
        target: FixedDotIndex,
        band: BandFirstSegIndex,
    ) {
        let source_pinname = self
            .node_pinname(GenericNode::Primitive(source.into()))
            .unwrap()
            .to_string();
        let target_pinname = self
            .node_pinname(GenericNode::Primitive(target.into()))
            .unwrap()
            .to_string();
        self.pinname_pair_to_band
            .insert((source_pinname, target_pinname), band);
    }

    pub fn band_between_pins(&self, pinname1: &str, pinname2: &str) -> Option<BandFirstSegIndex> {
        if let Some(band) = self
            .pinname_pair_to_band
            .get(&(pinname1.to_string(), pinname2.to_string()))
        {
            Some(*band)
        } else if let Some(band) = self
            .pinname_pair_to_band
            .get(&(pinname2.to_string(), pinname1.to_string()))
        {
            Some(*band)
        } else {
            None
        }
    }

    pub fn mesadata(&self) -> &M {
        self.layout.drawing().rules()
    }

    pub fn layout(&self) -> &Layout<M> {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout<M> {
        &mut self.layout
    }
}
