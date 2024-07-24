use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    board::{mesadata::AccessMesadata, BandName, Board},
    drawing::{
        band::BandUid,
        graph::{GetLayer, MakePrimitive, PrimitiveIndex},
    },
    geometry::{compound::ManageCompounds, GenericNode},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{poly::PolyWeight, CompoundWeight, NodeIndex},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PinSelector {
    pin: String,
    layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinSelection {
    selectors: HashSet<PinSelector>,
}

impl PinSelection {
    pub fn new() -> Self {
        Self {
            selectors: HashSet::new(),
        }
    }

    fn node_selector(
        &self,
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
            board.node_pinname(node),
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

    fn select(&mut self, selector: PinSelector) {
        self.selectors.insert(selector);
    }

    fn deselect(&mut self, selector: &PinSelector) {
        self.selectors.remove(selector);
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        self.node_selector(board, node)
            .map_or(false, |selector| self.selectors.contains(&selector))
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BandSelector {
    band: BandName,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandSelection {
    selectors: HashSet<BandSelector>,
}

impl BandSelection {
    pub fn new() -> Self {
        Self {
            selectors: HashSet::new(),
        }
    }

    fn node_selector(
        &self,
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
                .band_bandname(board.layout().drawing().collect().loose_band_uid(loose))?
                .clone(),
        })
    }

    fn select(&mut self, selector: BandSelector) {
        self.selectors.insert(selector);
    }

    fn deselect(&mut self, selector: &BandSelector) {
        self.selectors.remove(&selector);
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        self.node_selector(board, node)
            .map_or(false, |selector| self.selectors.contains(&selector))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub pin_selection: PinSelection,
    pub band_selection: BandSelection,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            pin_selection: PinSelection::new(),
            band_selection: BandSelection::new(),
        }
    }

    pub fn toggle_at_node(&mut self, board: &Board<impl AccessMesadata>, node: NodeIndex) {
        if let Some(selector) = self.pin_selection.node_selector(board, node) {
            if self.pin_selection.contains_node(board, node) {
                self.pin_selection.deselect(&selector);
            } else {
                self.pin_selection.select(selector);
            }
        } else if let Some(selector) = self.band_selection.node_selector(board, node) {
            if self.band_selection.contains_node(board, node) {
                self.band_selection.deselect(&selector);
            } else {
                self.band_selection.select(selector);
            }
        }
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        self.pin_selection.contains_node(board, node)
            || self.band_selection.contains_node(board, node)
    }
}
