use petgraph::graph::EdgeIndex;
use serde::{Deserialize, Serialize};
use spade::InsertionError;
use thiserror::Error;

use crate::{
    board::{mesadata::AccessMesadata, Board},
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, Infringement},
    layout::via::ViaWeight,
    router::{navmesh::NavmeshError, RouterError, RouterOptions},
    triangulation::GetTrianvertexNodeIndex,
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    ratsnest::{Ratsnest, RatvertexIndex},
    remove_bands::RemoveBandsExecutionStepper,
    selection::{BandSelection, PinSelection},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutorouterOptions {
    pub presort_by_pairwise_detours: bool,
    pub router_options: RouterOptions,
}

#[derive(Error, Debug, Clone)]
pub enum AutorouterError {
    #[error("nothing to route")]
    NothingToRoute,
    #[error(transparent)]
    Navmesh(#[from] NavmeshError),
    #[error(transparent)]
    Router(#[from] RouterError),
    #[error("could not place via")]
    CouldNotPlaceVia(#[from] Infringement),
    #[error("could not remove band")]
    CouldNotRemoveBand(BandTermsegIndex),
    #[error("need exactly two ratlines")]
    NeedExactlyTwoRatlines,
}

pub struct Autorouter<M: AccessMesadata> {
    pub(super) board: Board<M>,
    pub(super) ratsnest: Ratsnest,
}

impl<M: AccessMesadata> Autorouter<M> {
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(board.layout())?;
        Ok(Self { board, ratsnest })
    }

    pub fn autoroute(
        &mut self,
        selection: &PinSelection,
        options: AutorouterOptions,
    ) -> Result<AutorouteExecutionStepper, AutorouterError> {
        self.autoroute_ratlines(self.selected_ratlines(selection), options)
    }

    pub(super) fn autoroute_ratlines(
        &mut self,
        ratlines: Vec<EdgeIndex<usize>>,
        options: AutorouterOptions,
    ) -> Result<AutorouteExecutionStepper, AutorouterError> {
        AutorouteExecutionStepper::new(self, ratlines, options)
    }

    pub fn undo_autoroute(&mut self, selection: &PinSelection) -> Result<(), AutorouterError> {
        self.undo_autoroute_ratlines(self.selected_ratlines(selection))
    }

    pub(super) fn undo_autoroute_ratlines(
        &mut self,
        ratlines: Vec<EdgeIndex<usize>>,
    ) -> Result<(), AutorouterError> {
        for ratline in ratlines.iter() {
            let band = self
                .ratsnest
                .graph()
                .edge_weight(*ratline)
                .unwrap()
                .band_termseg
                .unwrap();
            self.board
                .layout_mut()
                .remove_band(band)
                .map_err(|_| AutorouterError::CouldNotRemoveBand(band))?;
        }

        Ok(())
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
        ratline1: EdgeIndex<usize>,
        ratline2: EdgeIndex<usize>,
        options: AutorouterOptions,
    ) -> Result<CompareDetoursExecutionStepper, AutorouterError> {
        CompareDetoursExecutionStepper::new(self, ratline1, ratline2, options)
    }

    pub fn measure_length(
        &mut self,
        selection: &BandSelection,
    ) -> Result<MeasureLengthExecutionStepper, AutorouterError> {
        MeasureLengthExecutionStepper::new(selection)
    }

    pub fn ratline_endpoints(
        &mut self,
        ratline: EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let (source, target) = self.ratsnest.graph().edge_endpoints(ratline).unwrap();

        let source_dot = match self
            .ratsnest
            .graph()
            .node_weight(source)
            .unwrap()
            .node_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Poly(poly) => self.board.poly_apex(poly),
        };

        let target_dot = match self
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .node_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Poly(poly) => self.board.poly_apex(poly),
        };

        (source_dot, target_dot)
    }

    pub(super) fn selected_ratlines(&self, selection: &PinSelection) -> Vec<EdgeIndex<usize>> {
        self.ratsnest
            .graph()
            .edge_indices()
            .filter(|ratline| {
                let (source, target) = self.ratsnest.graph().edge_endpoints(*ratline).unwrap();

                let source_navvertex = self
                    .ratsnest
                    .graph()
                    .node_weight(source)
                    .unwrap()
                    .node_index();
                let to_navvertex = self
                    .ratsnest
                    .graph()
                    .node_weight(target)
                    .unwrap()
                    .node_index();

                selection.contains_node(&self.board, source_navvertex.into())
                    && selection.contains_node(&self.board, to_navvertex.into())
            })
            .collect()
    }

    pub fn board(&self) -> &Board<M> {
        &self.board
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }
}
