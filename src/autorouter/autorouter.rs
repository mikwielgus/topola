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
    layout::{Layout, NodeIndex},
    router::{navmesh::Navmesh, Router, RouterObserverTrait, RoutingError},
    triangulation::GetVertexIndex,
};

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    navmesh: Option<Navmesh>, // Useful for debugging.
    cur_ratline: Option<EdgeIndex<usize>>,
}

impl Autoroute {
    pub fn new(
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        autorouter: &mut Autorouter<impl RulesTrait>,
    ) -> Option<Self> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_ratline) = ratlines_iter.next() else {
            return None;
        };

        let (source, target) = Self::ratline_endpoints(autorouter, cur_ratline);
        let layout = autorouter.layout.lock().unwrap();
        let navmesh = Some(Navmesh::new(&layout, source, target).ok()?);

        let this = Self {
            ratlines_iter,
            navmesh,
            cur_ratline: Some(cur_ratline),
        };

        Some(this)
    }

    pub fn next<R: RulesTrait>(
        &mut self,
        autorouter: &mut Autorouter<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> bool {
        let (new_navmesh, new_ratline) = if let Some(cur_ratline) = self.ratlines_iter.next() {
            let (source, target) = Self::ratline_endpoints(autorouter, cur_ratline);

            let layout = autorouter.layout.lock().unwrap();
            (
                Some(Navmesh::new(&layout, source, target).ok().unwrap()),
                Some(cur_ratline),
            )
        } else {
            (None, None)
        };

        let router = Router::new_from_navmesh(
            &mut autorouter.layout,
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
        );

        let Ok(band) = router.unwrap().route_band(100.0, observer) else {
            return false;
        };

        autorouter
            .ratsnest
            .assign_band_to_ratline(self.cur_ratline.unwrap(), band);
        self.cur_ratline = new_ratline;

        self.navmesh.is_some()
    }

    fn ratline_endpoints<R: RulesTrait>(
        autorouter: &Autorouter<R>,
        ratline: EdgeIndex<usize>,
    ) -> (FixedDotIndex, FixedDotIndex) {
        let mut layout = autorouter.layout.lock().unwrap();
        let (source, target) = autorouter.ratsnest.graph().edge_endpoints(ratline).unwrap();

        let source_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(source)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        let target_dot = match autorouter
            .ratsnest
            .graph()
            .node_weight(target)
            .unwrap()
            .vertex_index()
        {
            RatsnestVertexIndex::FixedDot(dot) => dot,
            RatsnestVertexIndex::Zone(zone) => layout.zone_apex(zone),
        };

        (source_dot, target_dot)
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
            self.layout.lock().unwrap().remove_band(band);
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

                selection.contains(source_vertex.into()) && selection.contains(to_vertex.into())
            })
            .collect()
    }

    pub fn layout(&self) -> &Arc<Mutex<Layout<R>>> {
        &self.layout
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }
}
