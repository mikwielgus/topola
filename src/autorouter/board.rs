use std::collections::HashMap;

use crate::{
    drawing::{
        dot::{FixedDotIndex, FixedDotWeight},
        graph::{GetLayer, GetMaybeNet, PrimitiveIndex},
        rules::RulesTrait,
        seg::{FixedSegIndex, FixedSegWeight},
        Infringement,
    },
    geometry::{shape::ShapeTrait, GenericNode},
    graph::GenericIndex,
    layout::{
        zone::{GetMaybeApex, MakePolyShape, ZoneWeight},
        Layout,
    },
    math::Circle,
};

pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<ZoneWeight>>;

#[derive(Debug)]
pub struct Board<R: RulesTrait> {
    layout: Layout<R>,
    node_to_pinname: HashMap<NodeIndex, String>,
    layer_to_layername: HashMap<u64, String>,
    net_to_netname: HashMap<usize, String>,
}

impl<R: RulesTrait> Board<R> {
    pub fn new(layout: Layout<R>) -> Self {
        Self {
            layout,
            node_to_pinname: HashMap::new(),
            layer_to_layername: HashMap::new(),
            net_to_netname: HashMap::new(),
        }
    }

    pub fn add_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        maybe_pin: Option<String>,
    ) -> Result<FixedDotIndex, Infringement> {
        let dot = self.layout.add_fixed_dot(weight)?;

        if let Some(ref pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(dot.into()), pin.clone());
        }

        Ok(dot)
    }

    pub fn add_zone_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> Result<FixedDotIndex, Infringement> {
        let dot = self.layout.add_zone_fixed_dot(weight, zone)?;

        if let Some(pin) = self.node_pinname(GenericNode::Compound(zone)) {
            self.node_to_pinname
                .insert(GenericNode::Primitive(dot.into()), pin.to_string());
        }

        Ok(dot)
    }

    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        maybe_pin: Option<String>,
    ) -> Result<FixedSegIndex, Infringement> {
        let seg = self.layout.add_fixed_seg(from, to, weight)?;

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin.to_string());
        }

        Ok(seg)
    }

    pub fn add_zone_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        zone: GenericIndex<ZoneWeight>,
    ) -> Result<FixedSegIndex, Infringement> {
        let seg = self.layout.add_zone_fixed_seg(from, to, weight, zone)?;

        if let Some(pin) = self.node_pinname(GenericNode::Compound(zone)) {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin.to_string());
        }

        Ok(seg)
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

    pub fn bename_layer(&mut self, layer: u64, layername: String) {
        self.layer_to_layername.insert(layer, layername);
    }

    pub fn bename_net(&mut self, net: usize, netname: String) {
        self.net_to_netname.insert(net, netname);
    }

    pub fn zone_apex(&mut self, zone: GenericIndex<ZoneWeight>) -> FixedDotIndex {
        if let Some(apex) = self.layout.zone(zone).maybe_apex() {
            apex
        } else {
            self.add_zone_fixed_dot(
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
            .unwrap()
        }
    }

    pub fn node_pinname(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_pinname.get(&node)
    }

    pub fn layername(&self, layer: u64) -> Option<&String> {
        self.layer_to_layername.get(&layer)
    }

    pub fn netname(&self, net: usize) -> Option<&String> {
        self.net_to_netname.get(&net)
    }

    pub fn layout(&self) -> &Layout<R> {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout<R> {
        &mut self.layout
    }
}
