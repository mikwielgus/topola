use petgraph::graph::EdgeIndex;
use spade::InsertionError;
use thiserror::Error;

use crate::{
    board::{mesadata::AccessMesadata, Board},
    drawing::{dot::FixedDotIndex, Infringement},
    layout::via::ViaWeight,
    router::{navmesh::NavmeshError, RouterError},
    triangulation::GetTrianvertexNodeIndex,
};

use super::{
    autoroute::Autoroute,
    compare::Compare,
    place_via::PlaceVia,
    ratsnest::{Ratsnest, RatvertexIndex},
    remove_bands::RemoveBands,
    selection::{BandSelection, PinSelection},
};

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

    pub fn autoroute(&mut self, selection: &PinSelection) -> Result<Autoroute, AutorouterError> {
        self.autoroute_ratlines(self.selected_ratlines(selection))
    }

    pub(super) fn autoroute_ratlines(
        &mut self,
        ratlines: Vec<EdgeIndex<usize>>,
    ) -> Result<Autoroute, AutorouterError> {
        Autoroute::new(self, ratlines)
    }

    pub fn undo_autoroute(&mut self, selection: &PinSelection) {
        self.undo_autoroute_ratlines(self.selected_ratlines(selection))
    }

    pub(super) fn undo_autoroute_ratlines(&mut self, ratlines: Vec<EdgeIndex<usize>>) {
        for ratline in ratlines.iter() {
            let band = self
                .ratsnest
                .graph()
                .edge_weight(*ratline)
                .unwrap()
                .band_termseg
                .unwrap();
            self.board.layout_mut().remove_band(band);
        }
    }

    pub fn place_via(&self, weight: ViaWeight) -> Result<PlaceVia, AutorouterError> {
        PlaceVia::new(weight)
    }

    pub fn undo_place_via(&mut self, _weight: ViaWeight) {
        todo!();
    }

    pub fn remove_bands(&self, selection: &BandSelection) -> Result<RemoveBands, AutorouterError> {
        RemoveBands::new(selection)
    }

    pub fn undo_remove_bands(&mut self, _selection: &BandSelection) {
        todo!();
    }

    pub fn compare(&mut self, selection: &PinSelection) -> Result<Compare, AutorouterError> {
        self.compare_ratlines(self.selected_ratlines(selection))
    }

    fn compare_ratlines(
        &mut self,
        ratlines: Vec<EdgeIndex<usize>>,
    ) -> Result<Compare, AutorouterError> {
        let ratline1 = *ratlines
            .get(0)
            .ok_or(AutorouterError::NeedExactlyTwoRatlines)?;
        let ratline2 = *ratlines
            .get(1)
            .ok_or(AutorouterError::NeedExactlyTwoRatlines)?;
        Compare::new(self, ratline1, ratline2)
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

    fn selected_ratlines(&self, selection: &PinSelection) -> Vec<EdgeIndex<usize>> {
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
