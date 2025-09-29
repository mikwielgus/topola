// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages relationship between pins and bands and the primitives and compounds
//! that consitutite them on the layout. And some more.

pub mod edit;

use edit::{BoardDataEdit, BoardEdit};
pub use specctra_core::mesadata::AccessMesadata;

use bimap::BiBTreeMap;
use derive_getters::Getters;
use geo::Point;

use crate::{
    bimapset::BiBTreeMapSet,
    drawing::{
        band::BandUid,
        dot::{FixedDotIndex, FixedDotWeight},
        graph::PrimitiveIndex,
        seg::{FixedSegIndex, FixedSegWeight},
        DrawingException, Infringement,
    },
    geometry::{edit::ApplyGeometryEdit, GenericNode, GetLayer},
    graph::{GenericIndex, MakeRef},
    layout::{poly::PolyWeight, via::ViaWeight, CompoundWeight, Layout, NodeIndex},
    router::ng::EtchedPath,
};

/// Represents a band between two pins.
pub type BandName = planar_incr_embed::navmesh::OrderedPair<String>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolvedSelector<'a> {
    Band { band_uid: BandUid },
    Pin { pin_name: &'a str, layer: usize },
}

impl<'a> ResolvedSelector<'a> {
    pub fn try_from_node(board: &'a Board<impl AccessMesadata>, node: NodeIndex) -> Option<Self> {
        use crate::{drawing::graph::MakePrimitiveRef, graph::GetIndex};

        let (layer, loose) = match node {
            NodeIndex::Primitive(primitive) => (
                primitive.primitive_ref(board.layout().drawing()).layer(),
                primitive.try_into().ok(),
            ),
            NodeIndex::Compound(compound) => {
                match board.layout().drawing().compound_weight(compound) {
                    CompoundWeight::Poly(..) => (
                        GenericIndex::<PolyWeight>::new(compound.index())
                            .ref_(board.layout())
                            .layer(),
                        None,
                    ),
                    _ => return None,
                }
            }
        };

        if let Some(pin_name) = board.node_pinname(&node) {
            Some(ResolvedSelector::Pin { pin_name, layer })
        } else {
            loose.and_then(|loose| {
                Some(ResolvedSelector::Band {
                    band_uid: board.layout().drawing().find_loose_band_uid(loose).ok()?,
                })
            })
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
    bands_by_id: BiBTreeMap<EtchedPath, BandUid>,
    // TODO: Simplify access logic to these members so that `#[getter(skip)]`s can be removed.
    #[getter(skip)]
    pinname_nodes: BiBTreeMapSet<String, NodeIndex>,
    #[getter(skip)]
    band_bandname: BiBTreeMap<BandUid, BandName>,
}

impl<M> Board<M> {
    /// Initializes the board with given [`Layout`]
    pub fn new(layout: Layout<M>) -> Self {
        Self {
            layout,
            bands_by_id: BiBTreeMap::new(),
            pinname_nodes: BiBTreeMapSet::new(),
            band_bandname: BiBTreeMap::new(),
        }
    }

    /// Returns a mutable reference to the layout, allowing modifications.
    pub fn layout_mut(&mut self) -> &mut Layout<M> {
        &mut self.layout
    }
}

impl<M: AccessMesadata> Board<M> {
    pub fn add_via(
        &mut self,
        recorder: &mut BoardEdit,
        weight: ViaWeight,
        maybe_pin: Option<String>,
    ) -> Result<(GenericIndex<ViaWeight>, Vec<FixedDotIndex>), Infringement> {
        let (weight, dots) = self.layout.add_via(&mut recorder.layout_edit, weight)?;

        if let Some(pin) = maybe_pin {
            for dot in dots.clone() {
                self.pinname_nodes
                    .insert(pin.clone(), GenericNode::Primitive(dot.into()));
            }
        }

        Ok((weight, dots))
    }

    /// Adds a new fixed dot with an optional pin name.
    ///
    /// Inserts the dot into the layout and, if a pin name is provided, maps it to the created dot's node.
    pub fn add_fixed_dot_infringably(
        &mut self,
        recorder: &mut BoardEdit,
        weight: FixedDotWeight,
        maybe_pin: Option<String>,
    ) -> FixedDotIndex {
        let dot = self
            .layout
            .add_fixed_dot_infringably(&mut recorder.layout_edit, weight);

        if let Some(pin) = maybe_pin {
            self.pinname_nodes
                .insert(pin, GenericNode::Primitive(dot.into()));
        }

        dot
    }

    /// Adds a fixed segment associated with a polygon in the layout.
    ///
    /// Adds the segment to the layout and updates the internal mapping if necessary.
    pub fn add_fixed_seg_infringably(
        &mut self,
        recorder: &mut BoardEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
        maybe_pin: Option<String>,
    ) -> FixedSegIndex {
        let seg =
            self.layout
                .add_fixed_seg_infringably(&mut recorder.layout_edit, from, to, weight);

        if let Some(pin) = maybe_pin {
            self.pinname_nodes
                .insert(pin, GenericNode::Primitive(seg.into()));
        }

        seg
    }

    /// Adds a new polygon to the layout with an optional pin name.
    ///
    /// Inserts the polygon into the layout and, if a pin name is provided, maps it to the created polygon's node.
    pub fn add_poly_with_nodes(
        &mut self,
        recorder: &mut BoardEdit,
        weight: PolyWeight,
        maybe_pin: Option<String>,
        nodes: &[PrimitiveIndex],
        fillets: &[FixedDotIndex],
    ) -> GenericIndex<PolyWeight> {
        let (poly, apex) =
            self.layout
                .add_poly_with_nodes(&mut recorder.layout_edit, weight, nodes, fillets);

        if let Some(pin) = maybe_pin {
            for i in nodes {
                self.pinname_nodes
                    .insert(pin.clone(), GenericNode::Primitive(*i));
            }

            self.pinname_nodes
                .insert(pin.clone(), GenericNode::Primitive(apex.into()));

            self.pinname_nodes
                .insert(pin, GenericNode::Compound(poly.into()));
        }

        poly
    }

    /// Returns an iterator over all the pin names.
    pub fn pinnames(&self) -> impl Iterator<Item = &String> + '_ {
        self.pinname_nodes.keys()
    }

    /// Returns an iterator over the set of all nodes associated with a given
    /// pin name.
    pub fn pinname_nodes(&self, pinname: &str) -> impl Iterator<Item = NodeIndex> + '_ {
        self.pinname_nodes
            .get_values(pinname)
            .into_iter()
            .flat_map(|set| set.iter().map(|node| *node))
    }

    /// Returns the pin name associated with a given node.
    pub fn node_pinname(&self, node: &NodeIndex) -> Option<&String> {
        self.pinname_nodes.get_key(node)
    }

    /// Returns the apex belonging to a given pin, if there is any.
    pub fn pin_apex(&self, pin: &str, layer: usize) -> Option<(FixedDotIndex, Point)> {
        self.pinname_nodes
            .get_values(pin)
            .map(|node| {
                node.iter()
                    .find_map(|node| self.layout().apex_of_compoundless_node(*node, layer))
            })
            .flatten()
    }

    /// Returns the band name associated with a given band.
    pub fn band_bandname(&self, band: &BandUid) -> Option<&BandName> {
        self.band_bandname.get_by_left(band)
    }

    /// Returns the unique id associated with a given band name.
    pub fn bandname_band(&self, bandname: &BandName) -> Option<&BandUid> {
        self.band_bandname.get_by_right(bandname)
    }

    /// Registers that a band is between the two nodes.
    pub fn try_set_band_between_nodes(
        &mut self,
        recorder: &mut BoardDataEdit,
        source: FixedDotIndex,
        target: FixedDotIndex,
        band: BandUid,
    ) -> bool {
        let source_pinname = self
            .node_pinname(&GenericNode::Primitive(source.into()))
            .unwrap()
            .to_string();
        let target_pinname = self
            .node_pinname(&GenericNode::Primitive(target.into()))
            .unwrap()
            .to_string();
        let bandname = BandName::from((source_pinname, target_pinname));

        if self.band_bandname.get_by_right(&bandname).is_some() {
            false
        } else {
            let ep = EtchedPath {
                end_points: (source, target).into(),
            };

            self.bands_by_id.insert(ep, band);
            recorder.bands_by_id.insert(ep, (None, Some(band)));

            self.band_bandname.insert(band, bandname.clone());
            recorder.bands_by_name.insert(bandname, (None, Some(band)));

            true
        }
    }

    pub fn band_between_nodes(
        &self,
        source: FixedDotIndex,
        target: FixedDotIndex,
    ) -> Option<BandUid> {
        let source_pinname = self
            .node_pinname(&GenericNode::Primitive(source.into()))
            .unwrap();
        let target_pinname = self
            .node_pinname(&GenericNode::Primitive(target.into()))
            .unwrap();
        self.band_between_pins(source_pinname, target_pinname)
    }

    /// Removes the band between the two nodes.
    pub fn remove_band_between_nodes(
        &mut self,
        recorder: &mut BoardEdit,
        source: FixedDotIndex,
        target: FixedDotIndex,
    ) -> Result<(), DrawingException> {
        let ep = EtchedPath {
            end_points: (source, target).into(),
        };
        let source_pinname = self
            .node_pinname(&GenericNode::Primitive(source.into()))
            .unwrap()
            .to_string();
        let target_pinname = self
            .node_pinname(&GenericNode::Primitive(target.into()))
            .unwrap()
            .to_string();

        let bandname = BandName::from((source_pinname, target_pinname));
        let maybe_band = self.band_bandname.get_by_right(&bandname).cloned();
        self.band_bandname.remove_by_right(&bandname);

        if let Some((_, uid)) = self.bands_by_id.remove_by_left(&ep) {
            let (from, _) = uid.into();
            self.layout.remove_band(&mut recorder.layout_edit, from)?;

            recorder
                .board_data_edit
                .bands_by_id
                .insert(ep, (Some(uid), None));
        }

        if let Some(uid) = maybe_band {
            recorder
                .board_data_edit
                .bands_by_name
                .insert(bandname, (Some(uid), None));
        }

        Ok(())
    }

    /// Removes the band between two nodes given by [`BandUid`].
    pub fn remove_band_by_id(
        &mut self,
        recorder: &mut BoardEdit,
        uid: BandUid,
    ) -> Result<(), DrawingException> {
        if let Some(ep) = self.bands_by_id.get_by_right(&uid) {
            let (source, target) = ep.end_points.into();
            self.remove_band_between_nodes(recorder, source, target)?;
        }

        Ok(())
    }

    /// Finds a band between two pin names.
    pub fn band_between_pins(&self, pinname1: &str, pinname2: &str) -> Option<BandUid> {
        self.band_bandname
            // note: it doesn't matter in what order pinnames are given, the constructor sorts them
            .get_by_right(&BandName::from((
                pinname1.to_string(),
                pinname2.to_string(),
            )))
            .copied()
    }

    pub fn apply_edit(&mut self, edit: &BoardEdit) {
        for (bandname, (maybe_old_band_uid, _)) in &edit.board_data_edit.bands_by_name {
            if maybe_old_band_uid.is_some() {
                self.band_bandname.remove_by_right(bandname);
            }
        }

        for (ep, (maybe_old_band_uid, _)) in &edit.board_data_edit.bands_by_id {
            if maybe_old_band_uid.is_some() {
                self.bands_by_id.remove_by_left(ep);
            }
        }

        self.layout_mut().apply(&edit.layout_edit);

        for (bandname, (_, maybe_new_band_uid)) in &edit.board_data_edit.bands_by_name {
            if let Some(band_uid) = maybe_new_band_uid {
                self.band_bandname.insert(*band_uid, bandname.clone());
            }
        }

        for (ep, (_, maybe_new_band_uid)) in &edit.board_data_edit.bands_by_id {
            if let Some(band_uid) = maybe_new_band_uid {
                self.bands_by_id.insert(*ep, *band_uid);
            }
        }
    }

    /// Returns the mesadata associated with the layout's drawing rules.
    pub fn mesadata(&self) -> &M {
        self.layout.drawing().rules()
    }
}
