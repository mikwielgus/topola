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
        ratsnest::{Ratsnest, RatvertexIndex},
        selection::Selection,
    },
    board::{mesadata::MesadataTrait, Board},
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet},
    },
    layout::Layout,
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

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    navmesh: Option<Navmesh>, // Useful for debugging.
    cur_ratline: Option<EdgeIndex<usize>>,
}

impl Autoroute {
    pub fn new(
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        autorouter: &mut Autorouter<impl MesadataTrait>,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = autorouter.ratline_endpoints(cur_ratline);
        let navmesh = Some(Navmesh::new(autorouter.board.layout(), source, target)?);

        let this = Self {
            ratlines_iter,
            navmesh,
            cur_ratline: Some(cur_ratline),
        };

        Ok(this)
    }

    pub fn step<M: MesadataTrait>(
        &mut self,
        autorouter: &mut Autorouter<M>,
        observer: &mut impl RouterObserverTrait<M>,
    ) -> Result<AutorouterStatus, AutorouterError> {
        let (new_navmesh, new_ratline) = if let Some(cur_ratline) = self.ratlines_iter.next() {
            let (source, target) = autorouter.ratline_endpoints(cur_ratline);

            (
                Some(
                    Navmesh::new(autorouter.board.layout(), source, target)
                        .ok()
                        .unwrap(),
                ),
                Some(cur_ratline),
            )
        } else {
            (None, None)
        };

        match autorouter.board.route_band(
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
            100.0,
            observer,
        ) {
            Ok(band) => {
                autorouter
                    .ratsnest
                    .assign_band_to_ratline(self.cur_ratline.unwrap(), band);
                self.cur_ratline = new_ratline;

                if self.navmesh.is_some() {
                    Ok(AutorouterStatus::Running)
                } else {
                    Ok(AutorouterStatus::Finished)
                }
            }
            Err(err) => Err(AutorouterError::Router(err)),
        }
    }

    pub fn navmesh(&self) -> &Option<Navmesh> {
        &self.navmesh
    }
}

pub struct Autorouter<M: MesadataTrait> {
    board: Board<M>,
    ratsnest: Ratsnest,
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
