use petgraph::graph::EdgeIndex;
use spade::InsertionError;
use thiserror::Error;

use crate::{
    autorouter::{
        autoroute::Autoroute,
        place_via::PlaceVia,
        ratsnest::{Ratsnest, RatvertexIndex},
        selection::Selection,
    },
    board::{mesadata::AccessMesadata, Board},
    drawing::{dot::FixedDotIndex, Infringement},
    layout::via::ViaWeight,
    router::{navmesh::NavmeshError, RouterError},
    triangulation::GetTrianvertexNodeIndex,
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
    Finished,
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

    pub fn autoroute(&mut self, selection: &Selection) -> Result<(), AutorouterError> {
        let mut autoroute = self.autoroute_walk(selection)?;

        loop {
            let status = match autoroute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let AutorouterStatus::Finished = status {
                return Ok(());
            }
        }
    }

    pub fn autoroute_walk(&mut self, selection: &Selection) -> Result<Autoroute, AutorouterError> {
        Autoroute::new(self, self.selected_ratlines(selection))
    }

    pub fn undo_autoroute(&mut self, selection: &Selection) {
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

    pub fn place_via(&mut self, weight: ViaWeight) -> Result<(), AutorouterError> {
        self.board.layout_mut().add_via(weight)?;
        Ok(())
    }

    pub fn place_via_walk(&self, weight: ViaWeight) -> Result<PlaceVia, AutorouterError> {
        PlaceVia::new(weight)
    }

    pub fn undo_place_via(&mut self, weight: ViaWeight) {
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

    fn selected_ratlines(&self, selection: &Selection) -> Vec<EdgeIndex<usize>> {
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
