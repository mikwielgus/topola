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
    autorouter::{
        multilayer_autoroute::{MultilayerAutorouteExecutionStepper, MultilayerAutorouteOptions},
        planar_reconfigurator::PlanarAutorouteReconfigurator,
        planner::Planner,
        ratsnests::Ratsnests,
    },
    board::{AccessMesadata, Board},
    drawing::{band::BandTermsegIndex, graph::MakePrimitiveRef},
    geometry::GetLayer,
    graph::MakeRef,
    layout::{via::ViaWeight, LayoutEdit, LayoutException},
    router::{navmesh::NavmeshError, ng, thetastar::ThetastarError, RouterOptions},
    triangulation::GetTrianvertexNodeIndex,
};

use super::{
    compare_detours::CompareDetoursExecutionStepper,
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    planar_autoroute::PlanarAutorouteExecutionStepper,
    pointroute::PointrouteExecutionStepper,
    ratline::RatlineUid,
    ratsnest::RatvertexNodeIndex,
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PresortBy {
    RatlineIntersectionCountAndLength,
    PairwiseDetours,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct PlanarAutorouteOptions {
    pub principal_layer: usize,
    pub presort_by: PresortBy,
    pub permutate: bool,
    pub router: RouterOptions,
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
    CouldNotPlaceVia(#[from] LayoutException),
    #[error("could not remove band")]
    CouldNotRemoveBand(BandTermsegIndex),
    #[error("need exactly two ratlines")]
    NeedExactlyTwoRatlines,
    #[error("nothing to undo for permutation")]
    NothingToUndoForReconfiguration,
}

#[derive(Getters)]
pub struct Autorouter<M> {
    pub(super) board: Board<M>,
    pub(super) ratsnests: Ratsnests,
}

impl<M: AccessMesadata> Autorouter<M> {
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        let ratsnests = Ratsnests::new(&board)?;
        Ok(Self { board, ratsnests })
    }

    pub fn pointroute(
        &mut self,
        selection: &PinSelection,
        point: Point,
        options: PlanarAutorouteOptions,
    ) -> Result<PointrouteExecutionStepper, AutorouterError> {
        let ratvertex = self
            .find_selected_ratvertex(selection, options.principal_layer)
            .unwrap();
        let origin_dot = match self
            .ratsnests
            .on_principal_layer_mut(options.principal_layer)
            .graph()
            .node_weight(ratvertex)
            .unwrap()
            .node_index()
        {
            RatvertexNodeIndex::FixedDot(dot) => dot,
            RatvertexNodeIndex::Poly(poly) => poly.ref_(self.board.layout()).apex(),
        };

        PointrouteExecutionStepper::new(self, origin_dot, point, options)
    }

    pub fn undo_pointroute(&mut self, band: BandTermsegIndex) -> Result<(), AutorouterError> {
        self.board
            .layout_mut()
            .remove_band(&mut LayoutEdit::new(), band)
            .map_err(|_| AutorouterError::CouldNotRemoveBand(band))
    }

    pub fn multilayer_autoroute(
        &mut self,
        selection: &PinSelection,
        options: MultilayerAutorouteOptions,
    ) -> Result<MultilayerAutorouteExecutionStepper, AutorouterError> {
        let planner = Planner::new(
            self,
            &self.selected_ratlines(selection, options.planar.principal_layer),
        );

        MultilayerAutorouteExecutionStepper::new(
            self,
            self.selected_ratlines(selection, options.planar.principal_layer),
            planner.plan().clone(),
            options,
        )
    }

    pub fn planar_autoroute(
        &mut self,
        selection: &PinSelection,
        options: PlanarAutorouteOptions,
    ) -> Result<PlanarAutorouteReconfigurator, AutorouterError> {
        PlanarAutorouteReconfigurator::new(
            self,
            self.selected_planar_ratlines(selection, options.principal_layer),
            options,
        )
    }

    pub(super) fn planar_autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineUid>,
        options: PlanarAutorouteOptions,
    ) -> Result<PlanarAutorouteExecutionStepper, AutorouterError> {
        PlanarAutorouteExecutionStepper::new(self, ratlines, options)
    }

    pub(super) fn undo_planar_autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineUid>,
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
            self.selected_ratlines(selection, active_layer),
            allowed_edges,
            active_layer,
            width,
            init_navmesh,
        )
    }

    pub(super) fn topo_autoroute_ratlines(
        &mut self,
        ratlines: Vec<RatlineUid>,
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
                let (origin, destination) = ratline.ref_(self).terminating_dots();

                if navmesh
                    .as_ref()
                    .node_data(&NavmeshIndex::Primal(origin))
                    .is_none()
                    || navmesh
                        .as_ref()
                        .node_data(&NavmeshIndex::Primal(destination))
                        .is_none()
                {
                    // e.g. due to wrong active layer
                    return None;
                }

                if self.board.band_between_nodes(origin, destination).is_some() {
                    // already connected
                    return None;
                }

                got_any_valid_goals = true;

                Some(ng::Goal {
                    source: origin,
                    target: destination,
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
        options: PlanarAutorouteOptions,
    ) -> Result<CompareDetoursExecutionStepper, AutorouterError> {
        let ratlines = self.selected_ratlines(selection, options.principal_layer);
        if ratlines.len() < 2 {
            return Err(AutorouterError::NeedExactlyTwoRatlines);
        }
        self.compare_detours_ratlines(ratlines[0], ratlines[1], options)
    }

    pub(super) fn compare_detours_ratlines(
        &mut self,
        ratline1: RatlineUid,
        ratline2: RatlineUid,
        options: PlanarAutorouteOptions,
    ) -> Result<CompareDetoursExecutionStepper, AutorouterError> {
        CompareDetoursExecutionStepper::new(self, ratline1, ratline2, options)
    }

    pub fn measure_length(
        &self,
        selection: &BandSelection,
    ) -> Result<MeasureLengthExecutionStepper, AutorouterError> {
        MeasureLengthExecutionStepper::new(selection)
    }

    pub(super) fn selected_ratlines(
        &self,
        selection: &PinSelection,
        principal_layer: usize,
    ) -> Vec<RatlineUid> {
        self.ratsnests()
            .on_principal_layer(principal_layer)
            .graph()
            .edge_indices()
            .filter(|index| {
                let (source, target) = self
                    .ratsnests()
                    .on_principal_layer(principal_layer)
                    .graph()
                    .edge_endpoints(*index)
                    .unwrap();

                let source_ratvertex = self
                    .ratsnests()
                    .on_principal_layer(principal_layer)
                    .graph()
                    .node_weight(source)
                    .unwrap()
                    .node_index();
                let to_ratvertex = self
                    .ratsnests()
                    .on_principal_layer(principal_layer)
                    .graph()
                    .node_weight(target)
                    .unwrap()
                    .node_index();

                selection.contains_node(&self.board, source_ratvertex.into())
                    && selection.contains_node(&self.board, to_ratvertex.into())
            })
            .map(|index| RatlineUid {
                principal_layer,
                index,
            })
            .collect()
    }

    fn selected_planar_ratlines(&self, selection: &PinSelection, layer: usize) -> Vec<RatlineUid> {
        self.selected_ratlines(selection, layer)
            .into_iter()
            .filter(|ratline| {
                let (endpoint_dot1, endpoint_dot2) = ratline.ref_(self).endpoint_dots();

                endpoint_dot1
                    .primitive_ref(self.board().layout().drawing())
                    .layer()
                    == layer
                    && endpoint_dot2
                        .primitive_ref(self.board().layout().drawing())
                        .layer()
                        == layer
            })
            .collect()
    }

    fn find_selected_ratvertex(
        &self,
        selection: &PinSelection,
        principal_layer: usize,
    ) -> Option<NodeIndex<usize>> {
        self.ratsnests()
            .on_principal_layer(principal_layer)
            .graph()
            .node_indices()
            .find(|ratvertex| {
                selection.contains_node(
                    &self.board,
                    self.ratsnests()
                        .on_principal_layer(principal_layer)
                        .graph()
                        .node_weight(*ratvertex)
                        .unwrap()
                        .node_index()
                        .into(),
                )
            })
    }
}
