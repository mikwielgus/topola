use std::{
    collections::HashSet,
    iter::Peekable,
    sync::{Arc, Mutex},
};

use geo::Point;
use petgraph::{
    graph::{EdgeIndex, EdgeIndices},
    visit::{EdgeRef, IntoEdgeReferences},
};
use spade::InsertionError;
use thiserror::Error;

use crate::{
    autorouter::{
        autoroute::Autoroute,
        place_via::PlaceVia,
        ratsnest::{Ratsnest, RatvertexIndex},
        selection::Selection,
    },
    board::{mesadata::MesadataTrait, Board},
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet},
    },
    layout::{via::ViaWeight, Layout},
    math::Circle,
    router::{
        navmesh::{Navmesh, NavmeshError},
        Router, RouterError, RouterObserverTrait,
    },
    triangulation::GetTrianvertexIndex,
};

#[derive(Error, Debug, Clone)]
pub enum AutorouterError {
    #[error("nothing to route")]
    NothingToRoute,
    #[error(transparent)]
    Navmesh(#[from] NavmeshError),
    #[error(transparent)]
    Router(#[from] RouterError),
}

pub enum AutorouterStatus {
    Running,
    Finished,
}

pub struct Autorouter<M: MesadataTrait> {
    pub(super) board: Board<M>,
    pub(super) ratsnest: Ratsnest,
}

impl<M: MesadataTrait> Autorouter<M> {
    pub fn new(board: Board<M>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(board.layout())?;
        Ok(Self { board, ratsnest })
    }

    pub fn autoroute(
        &mut self,
        selection: &Selection,
        observer: &mut impl RouterObserverTrait<M>,
    ) -> Result<(), AutorouterError> {
        let mut autoroute = self.autoroute_walk(selection)?;

        loop {
            let status = match autoroute.step(self, observer) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let AutorouterStatus::Finished = status {
                return Ok(());
            }
        }
    }

    pub fn autoroute_walk(&mut self, selection: &Selection) -> Result<Autoroute, AutorouterError> {
        Autoroute::new(self.selected_ratlines(selection), self)
    }

    pub fn undo_autoroute(&mut self, selection: &Selection) {
        for ratline in self.selected_ratlines(selection).iter() {
            let band = self
                .ratsnest
                .graph()
                .edge_weight(*ratline)
                .unwrap()
                .band
                .unwrap();
            self.board.layout_mut().remove_band(band);
        }
    }

    pub fn place_via(&mut self, weight: ViaWeight) -> Result<(), AutorouterError> {
        self.board.layout_mut().add_via(weight);
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
            .trianvertex_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Zone(zone) => self.board.zone_apex(zone),
        };

        let target_dot = match self
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .trianvertex_index()
        {
            RatvertexIndex::FixedDot(dot) => dot,
            RatvertexIndex::Zone(zone) => self.board.zone_apex(zone),
        };

        (source_dot, target_dot)
    }

    fn selected_ratlines(&self, selection: &Selection) -> Vec<EdgeIndex<usize>> {
        self.ratsnest
            .graph()
            .edge_indices()
            .filter(|ratline| {
                let (source, target) = self.ratsnest.graph().edge_endpoints(*ratline).unwrap();

                let source_vertex = self
                    .ratsnest
                    .graph()
                    .node_weight(source)
                    .unwrap()
                    .trianvertex_index();
                let to_vertex = self
                    .ratsnest
                    .graph()
                    .node_weight(target)
                    .unwrap()
                    .trianvertex_index();

                selection.contains_node(&self.board, source_vertex.into())
                    && selection.contains_node(&self.board, to_vertex.into())
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
