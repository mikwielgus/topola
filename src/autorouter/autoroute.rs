use petgraph::graph::EdgeIndex;

use crate::{
    autorouter::{
        invoker::{GetMaybeNavmesh, GetMaybeTrace},
        Autorouter, AutorouterError, AutorouterStatus,
    },
    board::mesadata::MesadataTrait,
    router::{navmesh::Navmesh, route::Route, trace::Trace, Router, RouterStatus},
};

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    route: Option<Route>,
    cur_ratline: Option<EdgeIndex<usize>>,
}

impl Autoroute {
    pub fn new(
        autorouter: &mut Autorouter<impl MesadataTrait>,
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = autorouter.ratline_endpoints(cur_ratline);
        let mut router = Router::new(autorouter.board.layout_mut());

        let this = Self {
            ratlines_iter,
            cur_ratline: Some(cur_ratline),
            route: Some(router.route_walk(source, target, 100.0)?),
        };

        Ok(this)
    }

    pub fn step<M: MesadataTrait>(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<AutorouterStatus, AutorouterError> {
        let Some(ref mut route) = self.route else {
            // Shouldn't happen.
            return Ok(AutorouterStatus::Finished);
        };

        let Some(cur_ratline) = self.cur_ratline else {
            return Ok(AutorouterStatus::Finished);
        };

        let (source, target) = autorouter.ratline_endpoints(cur_ratline);

        let band = {
            let mut router = Router::new(autorouter.board.layout_mut());

            let RouterStatus::Finished(band) = route.step(&mut router)? else {
                return Ok(AutorouterStatus::Running);
            };
            band
        };

        autorouter
            .ratsnest
            .assign_band_to_ratline(self.cur_ratline.unwrap(), band);

        autorouter
            .board
            .try_set_band_between_nodes(source, target, band);

        let Some(new_ratline) = self.ratlines_iter.next() else {
            self.cur_ratline = None;
            return Ok(AutorouterStatus::Finished);
        };

        let (source, target) = autorouter.ratline_endpoints(new_ratline);
        let mut router = Router::new(autorouter.board.layout_mut());

        self.cur_ratline = Some(new_ratline);
        self.route = Some(router.route_walk(source, target, 100.0)?);

        Ok(AutorouterStatus::Running)
    }

    pub fn navmesh(&self) -> Option<&Navmesh> {
        self.route.as_ref().map(|route| route.navmesh())
    }

    pub fn trace(&self) -> Option<&Trace> {
        self.route.as_ref().map(|route| route.trace())
    }
}

impl GetMaybeNavmesh for Autoroute {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.route.as_ref().map(|route| route.navmesh())
    }
}

impl GetMaybeTrace for Autoroute {
    fn maybe_trace(&self) -> Option<&Trace> {
        self.route.as_ref().map(|route| route.trace())
    }
}
