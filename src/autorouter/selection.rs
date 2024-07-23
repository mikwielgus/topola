use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    board::{mesadata::AccessMesadata, Board},
    drawing::{
        band::BandUid,
        graph::{GetLayer, MakePrimitive, PrimitiveIndex},
    },
    geometry::{compound::ManageCompounds, GenericNode},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{poly::PolyWeight, CompoundWeight, NodeIndex},
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Selector {
    pin: String,
    layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    selectors: HashSet<Selector>,
    #[serde(skip)]
    selected_bands: HashSet<BandUid>,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            selectors: HashSet::new(),
            selected_bands: HashSet::new(),
        }
    }

    pub fn new_select_all(board: &Board<impl AccessMesadata>) -> Self {
        let mut this = Self::new();

        for node in board.layout().drawing().primitive_nodes() {
            if let Some(selector) = this.node_selector(board, GenericNode::Primitive(node)) {
                if !this.contains_node(board, GenericNode::Primitive(node)) {
                    this.select(board, selector);
                }
            }
        }

        this
    }

    pub fn new_select_layer(board: &Board<impl AccessMesadata>, layer: usize) -> Self {
        let mut this = Self::new();

        for node in board.layout().drawing().layer_primitive_nodes(layer) {
            if let Some(selector) = this.node_selector(board, GenericNode::Primitive(node)) {
                if !this.contains_node(board, GenericNode::Primitive(node)) {
                    this.select(board, selector);
                }
            }
        }

        this
    }

    pub fn toggle_at_node(&mut self, board: &Board<impl AccessMesadata>, node: NodeIndex) {
        if let Some(selector) = self.node_selector(board, node) {
            if self.contains_node(board, node) {
                self.deselect(board, &selector);
            } else {
                self.select(board, selector);
            }
        } else if let Some(band) = self.node_band(board, node) {
            if self.contains_node(board, node) {
                self.deselect_band(board, band);
            } else {
                self.select_band(board, band);
            }
        }
    }

    fn select(&mut self, _board: &Board<impl AccessMesadata>, selector: Selector) {
        self.selectors.insert(selector);
    }

    fn deselect(&mut self, _board: &Board<impl AccessMesadata>, selector: &Selector) {
        self.selectors.remove(selector);
    }

    fn select_band(&mut self, board: &Board<impl AccessMesadata>, band: BandUid) {
        self.selected_bands.insert(band);
    }

    fn deselect_band(&mut self, board: &Board<impl AccessMesadata>, band: BandUid) {
        self.selected_bands.remove(&band);
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        if let Some(selector) = self.node_selector(board, node) {
            self.selectors.contains(&selector)
        } else if let Some(band) = self.node_band(board, node) {
            self.selected_bands.contains(&band)
        } else {
            false
        }
    }

    fn node_band(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> Option<BandUid> {
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

        Some(board.layout().drawing().collect().loose_band_uid(loose))
    }

    fn node_selector(
        &self,
        board: &Board<impl AccessMesadata>,
        node: NodeIndex,
    ) -> Option<Selector> {
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
            Some(Selector {
                pin: pinname.to_string(),
                layer: layername.to_string(),
            })
        } else {
            None
        }
    }
}
