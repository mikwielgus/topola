use std::collections::HashMap;

use crate::{
    drawing::{
        dot::{FixedDotIndex, FixedDotWeight},
        graph::PrimitiveIndex,
        rules::RulesTrait,
        seg::{FixedSegIndex, FixedSegWeight},
        Infringement,
    },
    geometry::GenericNode,
    graph::GenericIndex,
    layout::{zone::ZoneWeight, Layout},
};

pub type NodeIndex = GenericNode<PrimitiveIndex, GenericIndex<ZoneWeight>>;

#[derive(Debug)]
pub struct Board<R: RulesTrait> {
    layout: Layout<R>,
    node_to_pin: HashMap<NodeIndex, String>,
}

impl<R: RulesTrait> Board<R> {
    pub fn new(layout: Layout<R>) -> Self {
        Self {
            layout,
            node_to_pin: HashMap::new(),
        }
    }

    pub fn add_fixed_dot(
        &mut self,
        weight: FixedDotWeight,
        maybe_pin: Option<String>,
    ) -> Result<FixedDotIndex, Infringement> {
        let dot = self.layout.add_fixed_dot(weight)?;

        if let Some(ref pin) = maybe_pin {
            self.node_to_pin
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

        if let Some(pin) = self.node_pin(GenericNode::Compound(zone)) {
            self.node_to_pin
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
            self.node_to_pin
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

        if let Some(pin) = self.node_pin(GenericNode::Compound(zone)) {
            self.node_to_pin
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
            self.node_to_pin
                .insert(GenericNode::Compound(zone.into()), pin.to_string());
        }

        zone
    }

    pub fn node_pin(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_pin.get(&node)
    }

    pub fn layout(&self) -> &Layout<R> {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout<R> {
        &mut self.layout
    }
}
