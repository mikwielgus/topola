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
        ratsnest::{Ratsnest, RatsnestVertexIndex},
        selection::Selection,
    },
    drawing::{
        dot::FixedDotIndex,
        graph::{GetLayer, GetMaybeNet},
        rules::RulesTrait,
    },
    layout::{Layout, NodeIndex},
    router::{
        navmesh::{Navmesh, NavmeshError},
        Router, RouterError, RouterObserverTrait,
    },
    triangulation::GetVertexIndex,
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
        autorouter: &mut Autorouter<impl RulesTrait>,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = Self::ratline_endpoints(autorouter, cur_ratline);
        let navmesh = Some(Navmesh::new(&autorouter.layout, source, target)?);

        let this = Self {
            ratlines_iter,
            navmesh,
            cur_ratline: Some(cur_ratline),
        };

        Ok(this)
    }

    pub fn step<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> Result<AutorouterStatus, AutorouterError> {
        let (new_navmesh, new_ratline) = if let Some(cur_ratline) = self.ratlines_iter.next() {
            let (source, target) = Self::ratline_endpoints(autorouter, cur_ratline);

            (
                Some(
                    Navmesh::new(&autorouter.layout, source, target)
                        .ok()
                        .unwrap(),
                ),
                Some(cur_ratline),
            )
        } else {
            (None, None)
        };

        let mut router = Router::new_from_navmesh(
            &mut autorouter.layout,
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
        );

        match router.route_band(100.0, observer) {
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

    fn ratline_endpoints<R: RulesTrait>(
        autorouter: &mut Autorouter<R>,
        ratline: EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let (source, target) = autorouter.ratsnest.graph().edge_endpoints(ratline).unwrap();

        let source_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(source)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => autorouter.layout.zone_apex(zone),
        };

        let target_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => autorouter.layout.zone_apex(zone),
        };

        (source_dot, target_dot)
    }

    pub fn navmesh(&self) -> &Option<Navmesh> {
        &self.navmesh
    }
}

pub struct Autorouter<R: RulesTrait> {
    layout: Layout<R>,
    ratsnest: Ratsnest,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Layout<R>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(&layout)?;
        Ok(Self { layout, ratsnest })
    }

    pub fn autoroute(
        &mut self,
        selection: &Selection,
        observer: &mut impl RouterObserverTrait<R>,
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
            self.layout.remove_band(band);
        }
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
                    .vertex_index();
                let to_vertex = self
                    .ratsnest
                    .graph()
                    .node_weight(target)
                    .unwrap()
                    .vertex_index();

                selection.contains_node(&self.layout, source_vertex.into())
                    && selection.contains_node(&self.layout, to_vertex.into())
            })
            .collect()
    }

    pub fn layout(&self) -> &Layout<R> {
        &self.layout
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }
}
