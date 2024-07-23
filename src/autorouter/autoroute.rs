use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, route::Route, trace::Trace, Router, RouterStatus},
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError, AutorouterStatus,
};

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    route: Option<Route>,
    curr_ratline: Option<EdgeIndex<usize>>,
}

impl Autoroute {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(curr_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = autorouter.ratline_endpoints(curr_ratline);
        let mut router = Router::new(autorouter.board.layout_mut());

        let this = Self {
            ratlines_iter,
            curr_ratline: Some(curr_ratline),
            route: Some(router.route_walk(source, target, 100.0)?),
        };

        Ok(this)
    }

    pub fn step<M: AccessMesadata>(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<AutorouterStatus, AutorouterError> {
        let Some(ref mut route) = self.route else {
            // Shouldn't happen.
            return Ok(AutorouterStatus::Finished);
        };

        let Some(curr_ratline) = self.curr_ratline else {
            return Ok(AutorouterStatus::Finished);
        };

        let (source, target) = autorouter.ratline_endpoints(curr_ratline);

        let band_last_seg = {
            let mut router = Router::new(autorouter.board.layout_mut());

            let RouterStatus::Finished(band_last_seg) = route.step(&mut router)? else {
                return Ok(AutorouterStatus::Running);
            };
            band_last_seg
        };

        let band = autorouter
            .board
            .layout()
            .drawing()
            .collect()
            .loose_band_uid(band_last_seg.into());

        autorouter
            .ratsnest
            .assign_band_termseg_to_ratline(self.curr_ratline.unwrap(), band_last_seg);

        autorouter
            .board
            .try_set_band_between_nodes(source, target, band);

        let Some(new_ratline) = self.ratlines_iter.next() else {
            self.curr_ratline = None;
            return Ok(AutorouterStatus::Finished);
        };

        let (source, target) = autorouter.ratline_endpoints(new_ratline);
        let mut router = Router::new(autorouter.board.layout_mut());

        self.curr_ratline = Some(new_ratline);
        self.route = Some(router.route_walk(source, target, 100.0)?);

        Ok(AutorouterStatus::Running)
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

impl GetGhosts for Autoroute {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.route.as_ref().map_or(&[], |route| route.ghosts())
    }
}

impl GetObstacles for Autoroute {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.route.as_ref().map_or(&[], |route| route.obstacles())
    }
}
