use petgraph::graph::EdgeIndex;
use spade::InsertionError;
use thiserror::Error;

use crate::{
    board::{mesadata::AccessMesadata, Board},
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, Infringement},
    layout::via::ViaWeight,
    router::{navmesh::NavmeshError, RouterError},
    triangulation::GetTrianvertexNodeIndex,
};

use super::{
    autoroute::Autoroute,
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
}

pub enum AutorouterStatus {
    Running,
    Routed(BandTermsegIndex),
    Finished,
}

impl TryInto<()> for AutorouterStatus {
    type Error = ();
    fn try_into(self) -> Result<(), ()> {
        match self {
            AutorouterStatus::Running => Err(()),
            AutorouterStatus::Routed(..) => Err(()),
            AutorouterStatus::Finished => Ok(()),
        }
    }
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
        Autoroute::new(self, self.selected_ratlines(selection))
    }

    pub fn undo_autoroute(&mut self, selection: &PinSelection) {
        for ratline in self.selected_ratlines(selection).iter() {
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
