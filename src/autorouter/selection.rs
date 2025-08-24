// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use rstar::AABB;
use serde::{Deserialize, Serialize};

use crate::{
    board::{AccessMesadata, BandName, Board, ResolvedSelector},
    drawing::graph::{MakePrimitive, PrimitiveIndex},
    geometry::{
        shape::{AccessShape, Shape},
        GenericNode, GetLayer,
    },
    graph::{GenericIndex, GetPetgraphIndex, MakeRef},
    layout::{poly::PolyWeight, CompoundWeight, NodeIndex},
};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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
                    GenericIndex::<PolyWeight>::new(compound.petgraph_index())
                        .ref_(board.layout())
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

    pub fn try_from_pin_and_layer_id(
        board: &Board<impl AccessMesadata>,
        pin: &str,
        layer: usize,
    ) -> Option<PinSelector> {
        Some(PinSelector {
            pin: pin.to_string(),
            layer: board.layout().rules().layer_layername(layer)?.to_string(),
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PinSelection(pub BTreeSet<PinSelector>);

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
        PinSelector::try_from_node(board, node).is_some_and(|selector| self.0.contains(&selector))
    }

    pub fn selectors(&self) -> impl Iterator<Item = &PinSelector> {
        self.0.iter()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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

        Self::try_from_uid(board, &board.layout().drawing().loose_band_uid(loose).ok()?)
    }

    pub fn try_from_uid(
        board: &Board<impl AccessMesadata>,
        uid: &crate::drawing::band::BandUid,
    ) -> Option<BandSelector> {
        Some(BandSelector {
            band: board.band_bandname(uid)?.clone(),
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BandSelection(BTreeSet<BandSelector>);

impl BandSelection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains_node(&self, board: &Board<impl AccessMesadata>, node: NodeIndex) -> bool {
        BandSelector::try_from_node(board, node).is_some_and(|selector| self.0.contains(&selector))
    }

    pub fn selectors(&self) -> impl Iterator<Item = &BandSelector> {
        self.0.iter()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BboxSelectionKind {
    CompletelyInside,
    MerelyIntersects,
}

impl BboxSelectionKind {
    pub fn matches(&self, bigger: &AABB<[f64; 2]>, smaller: &Shape) -> bool {
        use rstar::Envelope;
        match self {
            Self::CompletelyInside => bigger.contains_envelope(&smaller.bbox_without_margin()),
            Self::MerelyIntersects => smaller.intersects_with_bbox(bigger),
        }
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

    pub fn select_all_in_bbox(
        &mut self,
        board: &Board<impl AccessMesadata>,
        aabb: &AABB<[f64; 2]>,
        layers: &[bool],
        kind: BboxSelectionKind,
    ) {
        const INF: f64 = f64::INFINITY;
        let layout = board.layout();

        let resolved_selectors =
            match kind {
                BboxSelectionKind::CompletelyInside => {
                    // 1. gather relevant node indices, and group them by resolved selectors
                    // .0 collects all nodes per resolved selection
                    // .1 collects only nodes which are actively selected here
                    let mut selectors = BTreeMap::<
                        ResolvedSelector,
                        (BTreeSet<NodeIndex>, BTreeSet<NodeIndex>),
                    >::new();
                    for &geom in layout.drawing().rtree().locate_in_envelope_intersecting(
                        &AABB::<[f64; 3]>::from_corners([-INF, -INF, -INF], [INF, INF, INF]),
                    ) {
                        let node = geom.data;
                        if layout.drawing().is_node_in_any_layer_of(node, layers) {
                            if let Some(rsel) = ResolvedSelector::try_from_node(board, node) {
                                let rseli = selectors.entry(rsel).or_default();
                                rseli.0.insert(node);
                                if kind.matches(aabb, &layout.node_shape(node)) {
                                    rseli.1.insert(node);
                                }
                            }
                        }
                    }

                    // 2. restrict to complete matches, return associated keys
                    selectors
                        .into_iter()
                        .filter(|(_, nis)| nis.0 == nis.1)
                        .map(|(k, _)| k)
                        .collect::<BTreeSet<_>>()
                }
                BboxSelectionKind::MerelyIntersects => {
                    // 1. gather relevant resolved selectors
                    let mut selectors = BTreeSet::<ResolvedSelector>::new();
                    for &geom in layout.drawing().rtree().locate_in_envelope_intersecting(
                        &AABB::<[f64; 3]>::from_corners(
                            [aabb.lower()[0], aabb.lower()[1], -f64::INFINITY],
                            [aabb.upper()[0], aabb.upper()[1], f64::INFINITY],
                        ),
                    ) {
                        let node = geom.data;
                        if layout.drawing().is_node_in_any_layer_of(node, layers)
                            && kind.matches(aabb, &layout.node_shape(node))
                        {
                            if let Some(rsel) = ResolvedSelector::try_from_node(board, node) {
                                selectors.insert(rsel);
                            }
                        }
                    }
                    // 2. nothing to restrict
                    selectors
                }
            };

        // 3. convert resolved selectors to actual selections
        for i in resolved_selectors {
            match i {
                ResolvedSelector::Band { band_uid } => {
                    if let Some(x) = BandSelector::try_from_uid(board, &band_uid) {
                        self.band_selection.0.insert(x);
                    }
                }
                ResolvedSelector::Pin { pin_name, layer } => {
                    if let Some(x) = PinSelector::try_from_pin_and_layer_id(board, pin_name, layer)
                    {
                        self.pin_selection.0.insert(x);
                    }
                }
            }
        }
    }

    pub fn select_at_node(&mut self, board: &Board<impl AccessMesadata>, node: NodeIndex) {
        if let Some(selector) = PinSelector::try_from_node(board, node) {
            self.pin_selection.0.insert(selector);
        } else if let Some(selector) = BandSelector::try_from_node(board, node) {
            self.band_selection.0.insert(selector);
        }
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

impl<'a> core::ops::BitXorAssign<&'a Selection> for Selection {
    fn bitxor_assign(&mut self, rhs: &'a Selection) {
        self.pin_selection.0 = &self.pin_selection.0 ^ &rhs.pin_selection.0;
        self.band_selection.0 = &self.band_selection.0 ^ &rhs.band_selection.0;
    }
}
