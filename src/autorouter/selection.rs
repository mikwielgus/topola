use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    board::{mesadata::AccessMesadata, BandName, Board},
    drawing::graph::{GetLayer, MakePrimitive, PrimitiveIndex},
    geometry::{compound::ManageCompounds, GenericNode},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{poly::PolyWeight, CompoundWeight, NodeIndex},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PinSelector {
    pub pin: String,
    pub layer: String,
}

impl PinSelector {
    pub fn try_from_node(
        board: &Board<impl AccessMesadata>,
        node: NodeIndex,
    ) -> Option<PinSelector> {
        let layer = match node {
            NodeIndex::Primitive(primitive) => {
                primitive.primitive(board.layout().drawing()).layer()
            }
            NodeIndex::Compound(compound) => {
                if let CompoundWeight::Poly(..) = board.layout().drawing().compound_weight(compound)
                {
                    board
                        .layout()
                        .poly(GenericIndex::<PolyWeight>::new(compound.petgraph_index()))
                        .layer()
                } else {
                    unreachable!()
                }
            }
        };

        if let (Some(pinname), Some(layername)) = (
            board.node_pinname(&node),
            board.layout().rules().layer_layername(layer),
        ) {
            Some(PinSelector {
                pin: pinname.to_string(),
                layer: layername.to_string(),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PinSelection(HashSet<PinSelector>);

impl PinSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_select_layer(board: &Board<impl AccessMesadata>, layer: usize) -> Self {
        let mut this = Self::default();

        for node in board.layout().drawing().layer_primitive_nodes(layer) {
            if let Some(selector) = PinSelector::try_from_node(board, GenericNode::Primitive(node))
            {
                this.0.insert(selector);
            }
        }

        this
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        PinSelector::try_from_node(board, node).map_or(false, |selector| self.0.contains(&selector))
    }

    pub fn selectors(&self) -> impl Iterator<Item = &PinSelector> {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BandSelector {
    pub band: BandName,
}

impl BandSelector {
    pub fn try_from_node(
        board: &Board<impl AccessMesadata>,
        node: NodeIndex,
    ) -> Option<BandSelector> {
        let NodeIndex::Primitive(primitive) = node else {
            return None;
        };

        let loose = match primitive {
            PrimitiveIndex::LooseDot(dot) => dot.into(),
            PrimitiveIndex::LoneLooseSeg(seg) => seg.into(),
            PrimitiveIndex::SeqLooseSeg(seg) => seg.into(),
            PrimitiveIndex::LooseBend(bend) => bend.into(),
            _ => return None,
        };

        Some(BandSelector {
            band: board
                .band_bandname(&board.layout().drawing().collect().loose_band_uid(loose))?
                .clone(),
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BandSelection(HashSet<BandSelector>);

impl BandSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        BandSelector::try_from_node(board, node)
            .map_or(false, |selector| self.0.contains(&selector))
    }

    pub fn selectors(&self) -> impl Iterator<Item = &BandSelector> {
        self.0.iter()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub pin_selection: PinSelection,
    pub band_selection: BandSelection,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle_at_node(&mut self, board: &Board<impl AccessMesadata>, node: NodeIndex) {
        if let Some(selector) = PinSelector::try_from_node(board, node) {
            if self.pin_selection.0.contains(&selector) {
                self.pin_selection.0.remove(&selector);
            } else {
                self.pin_selection.0.insert(selector);
            }
        } else if let Some(selector) = BandSelector::try_from_node(board, node) {
            if self.band_selection.0.contains(&selector) {
                self.band_selection.0.remove(&selector);
            } else {
                self.band_selection.0.insert(selector);
            }
        }
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        self.pin_selection.contains_node(board, node)
            || self.band_selection.contains_node(board, node)
    }
}
