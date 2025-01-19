// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Provides the functionality to create and manage relationships
//! between nodes, pins, and bands, as well as handle metadata and geometric data
//! for layout construction.

pub use specctra_core::mesadata::AccessMesadata;

use std::{cmp::Ordering, collections::BTreeMap};

use bimap::BiBTreeMap;
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

use crate::{
    drawing::{
        band::BandUid,
        bend::{BendIndex, BendWeight},
        dot::{DotIndex, DotWeight, FixedDotIndex, FixedDotWeight, GeneralDotWeight},
        graph::{GetMaybeNet, PrimitiveIndex, PrimitiveWeight},
        seg::{FixedSegIndex, FixedSegWeight, SegIndex, SegWeight},
        Collect,
    },
    geometry::{edit::ApplyGeometryEdit, shape::AccessShape, GenericNode, GetLayer},
    graph::GenericIndex,
    layout::{
        poly::{GetMaybeApex, MakePolygon, PolyWeight},
        CompoundWeight, Layout, LayoutEdit, NodeIndex,
    },
    math::Circle,
};

/// Represents a band between two pins.
#[derive(Clone, Debug, Deserialize, Eq, PartialOrd, Ord, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvedSelector<'a> {
    Band { band_uid: BandUid },
    Pin { pin_name: &'a str, layer: usize },
}

impl<'a> ResolvedSelector<'a> {
    pub fn try_from_node(board: &'a Board<impl AccessMesadata>, node: NodeIndex) -> Option<Self> {
        use crate::{drawing::graph::MakePrimitive, graph::GetPetgraphIndex};

        let (layer, loose) = match node {
            NodeIndex::Primitive(primitive) => (
                primitive.primitive(board.layout().drawing()).layer(),
                match primitive {
                    PrimitiveIndex::LooseDot(dot) => Some(dot.into()),
                    PrimitiveIndex::LoneLooseSeg(seg) => Some(seg.into()),
                    PrimitiveIndex::SeqLooseSeg(seg) => Some(seg.into()),
                    PrimitiveIndex::LooseBend(bend) => Some(bend.into()),
                    _ => None,
                },
            ),
            NodeIndex::Compound(compound) => {
                match board.layout().drawing().compound_weight(compound) {
                    CompoundWeight::Poly(..) => (
                        board
                            .layout()
                            .poly(GenericIndex::<PolyWeight>::new(compound.petgraph_index()))
                            .layer(),
                        None,
                    ),
                    _ => return None,
                }
            }
        };

        if let Some(pin_name) = board.node_pinname(&node) {
            Some(ResolvedSelector::Pin { pin_name, layer })
        } else if let Some(loose) = loose {
            Some(ResolvedSelector::Band {
                band_uid: board.layout().drawing().loose_band_uid(loose),
            })
        } else {
            None
        }
    }
}

/// Represents a board layout and its associated metadata.
///
/// The struct manages the relationships between board's layout,
/// and its compounds, as well as provides methods to manipulate them.
#[derive(Debug, Getters)]
pub struct Board<M> {
    layout: Layout<M>,
    // TODO: Simplify access logic to these members so that `#[getter(skip)]`s can be removed.
    #[getter(skip)]
    node_to_pinname: BTreeMap<NodeIndex, String>,
    #[getter(skip)]
    band_bandname: BiBTreeMap<BandUid, BandName>,
}

impl<M> Board<M> {
    /// Initializes the board with given [`Layout`]
    pub fn new(layout: Layout<M>) -> Self {
        Self {
            layout,
            node_to_pinname: BTreeMap::new(),
            band_bandname: BiBTreeMap::new(),
        }
    }

    /// Returns a mutable reference to the layout, allowing modifications.
    pub fn layout_mut(&mut self) -> &mut Layout<M> {
        &mut self.layout
    }
}

impl<M: AccessMesadata> Board<M> {
    /// Adds a new fixed dot with an optional pin name.
    ///
    /// Inserts the dot into the layout and, if a pin name is provided, maps it to the created dot's node.
    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        maybe_pin: Option<String>,
    ) -> FixedDotIndex {
        let dot = self.layout.add_fixed_dot_infringably(recorder, weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(dot.into()), pin);
        }

        dot
    }

    /// Adds a fixed segment between two dots with an optional pin name.
    ///
    /// Adds the segment to the layout and maps the pin name to the created segment if provided.
    pub fn add_poly_fixed_dot_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        weight: FixedDotWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let dot = self
            .layout
            .add_poly_fixed_dot_infringably(recorder, weight, poly);

        if let Some(pin) = self.node_pinname(&GenericNode::Compound(poly.into())) {
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
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        maybe_pin: Option<String>,
    ) -> FixedSegIndex {
        let seg = self
            .layout
            .add_fixed_seg_infringably(recorder, from, to, weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Primitive(seg.into()), pin);
        }

        seg
    }

    /// Adds a fixed segment associated with a polygon in the layout.
    ///
    /// Adds the segment to the layout and updates the internal mapping if necessary.
    pub fn add_poly_fixed_seg_infringably(
        &mut self,
        recorder: &mut LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedSegIndex {
        let seg = self
            .layout
            .add_poly_fixed_seg_infringably(recorder, from, to, weight, poly);

        if let Some(pin) = self.node_pinname(&GenericNode::Compound(poly.into())) {
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
        recorder: &mut LayoutEdit,
        weight: PolyWeight,
        maybe_pin: Option<String>,
    ) -> GenericIndex<PolyWeight> {
        let poly = self.layout.add_poly(recorder, weight);

        if let Some(pin) = maybe_pin {
            self.node_to_pinname
                .insert(GenericNode::Compound(poly.into()), pin);
        }

        poly
    }

    /// Retrieves or creates the apex (center point) of a polygon in the layout.
    ///
    /// If the polygon already has an apex, returns it. Otherwise, creates and returns a new fixed dot as the apex.
    pub fn poly_apex(
        &mut self,
        recorder: &mut LayoutEdit,
        poly: GenericIndex<PolyWeight>,
    ) -> FixedDotIndex {
        let resolved_poly = self.layout.poly(poly);
        if let Some(apex) = resolved_poly.maybe_apex() {
            apex
        } else {
            self.add_poly_fixed_dot_infringably(
                recorder,
                FixedDotWeight(GeneralDotWeight {
                    circle: Circle {
                        pos: resolved_poly.shape().center(),
                        r: 100.0,
                    },
                    layer: resolved_poly.layer(),
                    maybe_net: resolved_poly.maybe_net(),
                }),
                poly,
            )
        }
    }

    /// Returns the pin name associated with a given node.
    pub fn node_pinname(&self, node: &NodeIndex) -> Option<&String> {
        self.node_to_pinname.get(node)
    }

    /// Returns the band name associated with a given band.
    pub fn band_bandname(&self, band: &BandUid) -> Option<&BandName> {
        self.band_bandname.get_by_left(band)
    }

    /// Returns the unique id associated with a given band name.
    pub fn bandname_band(&self, bandname: &BandName) -> Option<&BandUid> {
        self.band_bandname.get_by_right(bandname)
    }

    /// Creates band between the two nodes
    pub fn try_set_band_between_nodes(
        &mut self,
        source: FixedDotIndex,
        target: FixedDotIndex,
        band: BandUid,
    ) {
        let source_pinname = self
            .node_pinname(&GenericNode::Primitive(source.into()))
            .unwrap()
            .to_string();
        let target_pinname = self
            .node_pinname(&GenericNode::Primitive(target.into()))
            .unwrap()
            .to_string();
        self.band_bandname
            .insert(band, BandName::new(source_pinname, target_pinname));
    }

    /// Finds a band between two pin names.
    pub fn band_between_pins(&self, pinname1: &str, pinname2: &str) -> Option<BandUid> {
        self.band_bandname
            // note: it doesn't matter in what order pinnames are given, the constructor sorts them
            .get_by_right(&BandName::new(pinname1.to_string(), pinname2.to_string()))
            .copied()
    }

    /// Returns the mesadata associated with the layout's drawing rules.
    pub fn mesadata(&self) -> &M {
        self.layout.drawing().rules()
    }
}

impl<M: AccessMesadata>
    ApplyGeometryEdit<
        PrimitiveWeight,
        DotWeight,
        SegWeight,
        BendWeight,
        CompoundWeight,
        PrimitiveIndex,
        DotIndex,
        SegIndex,
        BendIndex,
    > for Board<M>
{
    fn apply(&mut self, edit: &LayoutEdit) {
        self.layout.apply(edit);
    }
}
