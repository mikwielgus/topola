use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::{band::BandTermsegIndex, graph::PrimitiveIndex},
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, route::Route, trace::Trace, Router, RouterStatus},
    step::Step,
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError,
};

pub enum AutorouteStatus {
    Running,
    Routed(BandTermsegIndex),
    Finished,
}

impl TryInto<()> for AutorouteStatus {
    type Error = ();
    fn try_into(self) -> Result<(), ()> {
        match self {
            AutorouteStatus::Running => Err(()),
            AutorouteStatus::Routed(..) => Err(()),
            AutorouteStatus::Finished => Ok(()),
        }
    }
}

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
            route: Some(router.route(source, target, 100.0)?),
        };

        Ok(this)
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, AutorouteStatus, AutorouterError, ()> for Autoroute {
    fn step(&mut self, autorouter: &mut Autorouter<M>) -> Result<AutorouteStatus, AutorouterError> {
        let Some(curr_ratline) = self.curr_ratline else {
            return Ok(AutorouteStatus::Finished);
        };

        let Some(ref mut route) = self.route else {
            // Shouldn't happen.
            return Ok(AutorouteStatus::Finished);
        };

        let (source, target) = autorouter.ratline_endpoints(curr_ratline);

        let band_termseg = {
            let mut router = Router::new(autorouter.board.layout_mut());

            let RouterStatus::Finished(band_termseg) = route.step(&mut router)? else {
                return Ok(AutorouteStatus::Running);
            };
            band_termseg
        };

        let band = autorouter
            .board
            .layout()
            .drawing()
            .collect()
            .loose_band_uid(band_termseg.into());

        autorouter
            .ratsnest
            .assign_band_termseg_to_ratline(self.curr_ratline.unwrap(), band_termseg);

        autorouter
            .board
            .try_set_band_between_nodes(source, target, band);

        if let Some(new_ratline) = self.ratlines_iter.next() {
            let (source, target) = autorouter.ratline_endpoints(new_ratline);
            let mut router = Router::new(autorouter.board.layout_mut());

            self.curr_ratline = Some(new_ratline);
            self.route = Some(router.route(source, target, 100.0)?);
        } else {
            self.curr_ratline = None;
            //return Ok(AutorouteStatus::Finished);
        }

        Ok(AutorouteStatus::Routed(band_termseg))
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
