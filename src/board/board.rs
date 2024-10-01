use std::{cmp::Ordering, collections::HashMap};

use bimap::BiHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    board::mesadata::AccessMesadata,
    drawing::{
        band::BandUid,
        dot::{FixedDotIndex, FixedDotWeight},
        graph::{GetLayer, GetMaybeNet},
        seg::{FixedSegIndex, FixedSegWeight},
    },
    geometry::{shape::AccessShape, GenericNode},
    graph::GenericIndex,
    layout::{
        poly::{GetMaybeApex, MakePolyShape, PolyWeight},
        Layout, NodeIndex,
    },
    math::Circle,
};

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandName(String, String);

impl BandName {
    pub fn new(pinname1: String, pinname2: String) -> Self {
        if pinname1.cmp(&pinname2) == Ordering::Greater {
            BandName(pinname2, pinname1)
        } else {
            BandName(pinname1, pinname2)
        }
    }
}

#[derive(Debug)]
pub struct Board<M: AccessMesadata> {
    layout: Layout<M>,
    node_to_pinname: HashMap<NodeIndex, String>,
    band_bandname: BiHashMap<BandUid, BandName>,
}

impl<M: AccessMesadata> Board<M> {
    pub fn new(layout: Layout<M>) -> Self {
        Self {
            layout,
            node_to_pinname: HashMap::new(),
            band_bandname: BiHashMap::new(),
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

    pub fn add_poly_fixed_dot_infringably(
        &mut self,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let dot = self.layout.add_poly_fixed_dot_infringably(weight, poly);

        if let Some(pin) = self.node_pinname(GenericNode::Compound(poly.into())) {
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

    pub fn add_poly_fixed_seg_infringably(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedSegIndex {
        let seg = self
            .layout
            .add_poly_fixed_seg_infringably(from, to, weight, poly);

        if let Some(pin) = self.node_pinname(GenericNode::Compound(poly.into())) {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin.to_string());
        }

        seg
    }

    pub fn add_poly(
        &mut self,
        weight: PolyWeight,
        maybe_pin: Option<String>,
    ) -> GenericIndex<PolyWeight> {
        let poly = self.layout.add_poly(weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Compound(poly.into()), pin.to_string());
        }

        poly
    }

    pub fn poly_apex(&mut self, poly: GenericIndex<PolyWeight>) -> FixedDotIndex {
        if let Some(apex) = self.layout.poly(poly).maybe_apex() {
            apex
        } else {
            self.add_poly_fixed_dot_infringably(
                FixedDotWeight {
                    circle: Circle {
                        pos: self.layout.poly(poly).shape().center(),
                        r: 100.0,
                    },
                    layer: self.layout.poly(poly).layer(),
                    maybe_net: self.layout.poly(poly).maybe_net(),
                },
                poly,
            )
        }
    }

    pub fn node_pinname(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_pinname.get(&node)
    }

    pub fn band_bandname(&self, band: BandUid) -> Option<&BandName> {
        self.band_bandname.get_by_left(&band)
    }

    pub fn bandname_band(&self, bandname: BandName) -> Option<&BandUid> {
        self.band_bandname.get_by_right(&bandname)
    }

    pub fn try_set_band_between_nodes(
        &mut self,
        source: FixedDotIndex,
        target: FixedDotIndex,
        band: BandUid,
    ) {
        let source_pinname = self
            .node_pinname(GenericNode::Primitive(source.into()))
            .unwrap()
            .to_string();
        let target_pinname = self
            .node_pinname(GenericNode::Primitive(target.into()))
            .unwrap()
            .to_string();
        self.band_bandname
            .insert(band, BandName::new(source_pinname, target_pinname));
    }

    pub fn band_between_pins(&self, pinname1: &str, pinname2: &str) -> Option<BandUid> {
        if let Some(band) = self
            .band_bandname
            .get_by_right(&BandName::new(pinname1.to_string(), pinname2.to_string()))
        {
            Some(*band)
        } else if let Some(band) = self
            .band_bandname
            .get_by_right(&BandName::new(pinname2.to_string(), pinname1.to_string()))
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
