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
    layout::{connectivity::BandIndex, Layout, NodeIndex},
    router::{navmesh::Navmesh, Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    navmesh: Option<Navmesh>, // Useful for debugging.
}

impl Autoroute {
    pub fn new(
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        autorouter: &mut Autorouter<impl RulesTrait>,
    ) -> Option<Self> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_edge) = ratlines_iter.next() else {
            return None;
        };

        let (from, to) = Self::edge_from_to(autorouter, cur_edge);
        let layout = autorouter.layout.lock().unwrap();
        let navmesh = Some(Navmesh::new(&layout, from, to).ok()?);

        let this = Self {
            ratlines_iter,
            navmesh,
        };

        Some(this)
    }

    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> bool {
        let new_navmesh = if let Some(cur_edge) = self.ratlines_iter.next() {
            let (from, to) = Self::edge_from_to(autorouter, cur_edge);

            let layout = autorouter.layout.lock().unwrap();
            Some(Navmesh::new(&layout, from, to).ok().unwrap())
        } else {
            None
        };

        let router = Router::new_from_navmesh(
            &mut autorouter.layout,
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
        );
        router.unwrap().route_band(100.0, observer);

        self.navmesh.is_some()
    }

    fn edge_from_to<R: RulesTrait>(
        autorouter: &Autorouter<R>,
        edge: EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let mut layout = autorouter.layout.lock().unwrap();
        let (from, to) = autorouter.ratsnest.graph().edge_endpoints(edge).unwrap();

        let from_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(from)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        let to_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(to)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        (from_dot, to_dot)
    }

    pub fn navmesh(&self) -> &Option<Navmesh> {
        &self.navmesh
    }
}

pub struct Autorouter<R: RulesTrait> {
    layout: Arc<Mutex<Layout<R>>>,
    ratsnest: Ratsnest,
}

impl<R: RulesTrait> Autorouter<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Result<Self, InsertionError> {
        let ratsnest = Ratsnest::new(&layout.lock().unwrap())?;
        Ok(Self { layout, ratsnest })
    }

    pub fn autoroute(&mut self, selection: &Selection, observer: &mut impl RouterObserverTrait<R>) {
        if let Some(mut autoroute) = self.autoroute_walk(selection) {
            while autoroute.next(self, observer) {
                //
            }
        }
    }

    pub fn autoroute_walk(&mut self, selection: &Selection) -> Option<Autoroute> {
        Autoroute::new(
            self.ratsnest
                .graph()
                .edge_indices()
                .filter(|edge| {
                    let (from, to) = self.ratsnest.graph().edge_endpoints(*edge).unwrap();

                    let from_vertex = self
                        .ratsnest
                        .graph()
                        .node_weight(from)
                        .unwrap()
                        .vertex_index();
                    let to_vertex = self
                        .ratsnest
                        .graph()
                        .node_weight(to)
                        .unwrap()
                        .vertex_index();

                    selection.contains(&from_vertex.into()) && selection.contains(&to_vertex.into())
                })
                .collect::<Vec<_>>(),
            self,
        )
    }

    pub fn undo_autoroute(&mut self, selection: &Selection) {
        todo!();
    }

    pub fn layout(&self) -> &Arc<Mutex<Layout<R>>> {
        &self.layout
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }
}
