// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::Getters;
use geo::Point;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use spade::InsertionError;
use std::collections::BTreeSet;
use thiserror::Error;

use crate::{
    autorouter::permutator::AutorouteExecutionPermutator,
    board::{AccessMesadata, Board},
    drawing::{band::BandTermsegIndex, Infringement},
    graph::MakeRef,
    layout::{via::ViaWeight, LayoutEdit},
    router::{navmesh::NavmeshError, ng, thetastar::ThetastarError, RouterOptions},
    triangulation::GetTrianvertexNodeIndex,
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    pointroute::PointrouteExecutionStepper,
    ratline::RatlineIndex,
    ratsnest::{Ratsnest, RatvertexIndex},
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PresortBy {
    RatlineIntersectionCountAndLength,
    PairwiseDetours,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct AutorouterOptions {
    pub presort_by: PresortBy,
    pub permutate: bool,
    pub router_options: RouterOptions,
}

#[derive(Error, Debug, Clone)]
pub enum AutorouterError {
    #[error("nothing to route")]
    NothingToRoute,
    #[error(transparent)]
    Navmesh(#[from] NavmeshError),
    #[error("routing failed: {0}")]
    Thetastar(#[from] ThetastarError),
    #[error("TopoNavmesh generation failed: {0}")]
    TopoNavmeshGeneration(#[from] ng::NavmeshCalculationError),
    #[error("could not place via")]
    CouldNotPlaceVia(#[from] Infringement),
    #[error("could not remove band")]
    CouldNotRemoveBand(BandTermsegIndex),
    #[error("need exactly two ratlines")]
    NeedExactlyTwoRatlines,
    #[error("nothing to undo for permutation")]
    NothingToUndoForPermutation,
}

#[derive(Getters)]
pub struct Autorouter<M> {
    pub(super) board: Board<M>,
    pub(super) ratsnest: Ratsnest,
}

impl<M: AccessMesadata> Autorouter<M> {
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(board.layout())?;
        Ok(Self { board, ratsnest })
    }

    pub fn pointroute(
        &mut self,
        selection: &PinSelection,
        point: Point,
        options: AutorouterOptions,
    ) -> Result<PointrouteExecutionStepper, AutorouterError> {
        let ratvertex = self.find_selected_ratvertex(selection).unwrap();
        let origin_dot = match self
            .ratsnest
            .graph()
            .node_weight(ratvertex)
            .unwrap()
            .node_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Poly(poly) => poly.ref_(self.board.layout()).apex(),
        };

        PointrouteExecutionStepper::new(self, origin_dot, point, options)
    }

    pub fn undo_pointroute(&mut self, band: BandTermsegIndex) -> Result<(), AutorouterError> {
        self.board
            .layout_mut()
            .remove_band(&mut LayoutEdit::new(), band)
            .map_err(|_| AutorouterError::CouldNotRemoveBand(band))
    }

    pub fn autoroute(
        &mut self,
        selection: &PinSelection,
        options: AutorouterOptions,
    ) -> Result<AutorouteExecutionPermutator, AutorouterError> {
        AutorouteExecutionPermutator::new(self, self.selected_ratlines(selection), options)
    }

    pub(super) fn autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineIndex>,
        options: AutorouterOptions,
    ) -> Result<AutorouteExecutionStepper, AutorouterError> {
        AutorouteExecutionStepper::new(self, ratlines, options)
    }

    pub fn undo_autoroute(&mut self, selection: &PinSelection) -> Result<(), AutorouterError> {
        self.undo_autoroute_ratlines(self.selected_ratlines(selection))
    }

    pub(super) fn undo_autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineIndex>,
    ) -> Result<(), AutorouterError> {
        for ratline in ratlines.iter() {
            let band = ratline.ref_(self).band_termseg();
            self.board
                .layout_mut()
                .remove_band(&mut LayoutEdit::new(), band)
                .map_err(|_| AutorouterError::CouldNotRemoveBand(band))?;
        }

        Ok(())
    }

    pub fn topo_autoroute(
        &mut self,
        selection: &PinSelection,
        allowed_edges: BTreeSet<ng::PieEdgeIndex>,
        active_layer: usize,
        width: f64,
        init_navmesh: Option<ng::PieNavmesh>,
    ) -> Result<ng::AutorouteExecutionStepper<M>, AutorouterError>
    where
        M: Clone,
    {
        self.topo_autoroute_ratlines(
            self.selected_ratlines(selection),
            allowed_edges,
            active_layer,
            width,
            init_navmesh,
        )
    }

    pub(super) fn topo_autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineIndex>,
        allowed_edges: BTreeSet<ng::PieEdgeIndex>,
        active_layer: usize,
        width: f64,
        init_navmesh: Option<ng::PieNavmesh>,
    ) -> Result<ng::AutorouteExecutionStepper<M>, AutorouterError>
    where
        M: Clone,
    {
        let navmesh = if let Some(x) = init_navmesh {
            x
        } else {
            ng::calculate_navmesh(&self.board, active_layer)?
        };

        let mut got_any_valid_goals = false;

        use ng::pie::NavmeshIndex;

        let ret = ng::AutorouteExecutionStepper::new(
            self.board.layout(),
            &navmesh,
            self.board
                .bands_by_id()
                .iter()
                .map(|(&k, &v)| (k, v))
                .collect(),
            active_layer,
            allowed_edges,
            ratlines.into_iter().filter_map(|ratline| {
                let (source, target) = ratline.ref_(self).endpoint_dots();

                if navmesh
                    .as_ref()
                    .node_data(&NavmeshIndex::Primal(source))
                    .is_none()
                    || navmesh
                        .as_ref()
                        .node_data(&NavmeshIndex::Primal(target))
                        .is_none()
                {
                    // e.g. due to wrong active layer
                    return None;
                }

                if self.board.band_between_nodes(source, target).is_some() {
                    // already connected
                    return None;
                }

                got_any_valid_goals = true;

                Some(ng::Goal {
                    source,
                    target,
                    width,
                })
            }),
        );

        if !got_any_valid_goals {
            Err(AutorouterError::NothingToRoute)
        } else {
            Ok(ret)
        }
    }

    pub fn place_via(
        &self,
        weight: ViaWeight,
    ) -> Result<PlaceViaExecutionStepper, AutorouterError> {
        PlaceViaExecutionStepper::new(weight)
    }

    pub fn undo_place_via(&mut self, _weight: ViaWeight) {
        todo!();
    }

    pub fn remove_bands(
        &self,
        selection: &BandSelection,
    ) -> Result<RemoveBandsExecutionStepper, AutorouterError> {
        RemoveBandsExecutionStepper::new(selection)
    }

    pub fn undo_remove_bands(&mut self, _selection: &BandSelection) {
        todo!();
    }

    pub fn compare_detours(
        &mut self,
        selection: &PinSelection,
        options: AutorouterOptions,
    ) -> Result<CompareDetoursExecutionStepper, AutorouterError> {
        let ratlines = self.selected_ratlines(selection);
        if ratlines.len() < 2 {
            return Err(AutorouterError::NeedExactlyTwoRatlines);
        }
        self.compare_detours_ratlines(ratlines[0], ratlines[1], options)
    }

    pub(super) fn compare_detours_ratlines(
        &mut self,
        ratline1: RatlineIndex,
        ratline2: RatlineIndex,
        options: AutorouterOptions,
    ) -> Result<CompareDetoursExecutionStepper, AutorouterError> {
        CompareDetoursExecutionStepper::new(self, ratline1, ratline2, options)
    }

    pub fn measure_length(
        &self,
        selection: &BandSelection,
    ) -> Result<MeasureLengthExecutionStepper, AutorouterError> {
        MeasureLengthExecutionStepper::new(selection)
    }

    pub(super) fn selected_ratlines(&self, selection: &PinSelection) -> Vec<RatlineIndex> {
        self.ratsnest
            .graph()
            .edge_indices()
            .filter(|ratline| {
                let (source, target) = self.ratsnest.graph().edge_endpoints(*ratline).unwrap();

                let source_ratvertex = self
                    .ratsnest
                    .graph()
                    .node_weight(source)
                    .unwrap()
                    .node_index();
                let to_ratvertex = self
                    .ratsnest
                    .graph()
                    .node_weight(target)
                    .unwrap()
                    .node_index();

                selection.contains_node(&self.board, source_ratvertex.into())
                    && selection.contains_node(&self.board, to_ratvertex.into())
            })
            .collect()
    }

    fn find_selected_ratvertex(&self, selection: &PinSelection) -> Option<NodeIndex<usize>> {
        self.ratsnest.graph().node_indices().find(|ratvertex| {
            selection.contains_node(
                &self.board,
                self.ratsnest
                    .graph()
                    .node_weight(*ratvertex)
                    .unwrap()
                    .node_index()
                    .into(),
            )
        })
    }
}
