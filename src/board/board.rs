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

/// Represents a band between two pins.
#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandName(String, String);

impl BandName {
    /// Creates a new [`BandName`] and manages their order.
    ///
    /// This function ensures that the two pin names are sorted in lexicographical order, so that the smaller name always comes first.
    pub fn new(pinname1: String, pinname2: String) -> Self {
        if pinname1.cmp(&pinname2) == Ordering::Greater {
            BandName(pinname2, pinname1)
        } else {
            BandName(pinname1, pinname2)
        }
    }
}

/// Represents a board layout and its associated metadata.
///
/// The struct manages the relationships between board's layout,
/// and its compounds, as well as provides methods to manipulate them.
#[derive(Debug)]
pub struct Board<M: AccessMesadata> {
    layout: Layout<M>,
    node_to_pinname: HashMap<NodeIndex, String>,
    band_bandname: BiHashMap<BandUid, BandName>,
}

impl<M: AccessMesadata> Board<M> {
    /// Initializes the board with given [`Layout`]
    pub fn new(layout: Layout<M>) -> Self {
        Self {
            layout,
            node_to_pinname: HashMap::new(),
            band_bandname: BiHashMap::new(),
        }
    }

    /// Adds a new fixed dot with an optional pin name.
    ///
    /// Inserts the dot into the layout and, if a pin name is provided, maps it to the created dot's node.
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

    /// Adds a fixed segment between two dots with an optional pin name.
    /// 
    /// Adds the segment to the layout and maps the pin name to the created segment if provided.
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

    /// Adds a fixed segment associated with a polygon in the layout.
    /// 
    /// Adds the segment to the layout and updates the internal mapping if necessary.
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

    /// Adds a fixed segment associated with a polygon in the layout.
    ///
    /// Adds the segment to the layout and updates the internal mapping if necessary.
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
    
    /// Adds a new polygon to the layout with an optional pin name.
    ///
    /// Inserts the polygon into the layout and, if a pin name is provided, maps it to the created polygon's node.
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
    
    /// Retrieves or creates the apex (top point) of a polygon in the layout.
    ///
    /// If the polygon already has an apex, returns it. Otherwise, creates and returns a new fixed dot as the apex.
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

    /// Returns the pin name associated with a given node.
    pub fn node_pinname(&self, node: NodeIndex) -> Option<&String> {
        self.node_to_pinname.get(&node)
    }

    /// Returns the band name associated with a given band.
    pub fn band_bandname(&self, band: BandUid) -> Option<&BandName> {
        self.band_bandname.get_by_left(&band)
    }

    /// Returns the unique id associated with a given band name.
    pub fn bandname_band(&self, bandname: BandName) -> Option<&BandUid> {
        self.band_bandname.get_by_right(&bandname)
    }

    /// Creates band between the two nodes
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

    /// Finds a band between two pin names.
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

    /// Returns the mesadata associated with the layout's drawing rules.
    pub fn mesadata(&self) -> &M {
        self.layout.drawing().rules()
    }

    /// Returns the layout managed by this board.
    pub fn layout(&self) -> &Layout<M> {
        &self.layout
    }

    /// Returns a mutable reference to the layout, allowing modifications.
    pub fn layout_mut(&mut self) -> &mut Layout<M> {
        &mut self.layout
    }
}
