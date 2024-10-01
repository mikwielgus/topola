use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::{band::BandTermsegIndex, graph::PrimitiveIndex},
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, route::RouteStepper, trace::TraceStepper, Router, RouterStatus},
    stepper::Step,
};

use super::{
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError, AutorouterOptions,
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

pub struct AutorouteExecutionStepper {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    options: AutorouterOptions,
    route: Option<RouteStepper>,
    curr_ratline: Option<EdgeIndex<usize>>,
}

impl AutorouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(curr_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = autorouter.ratline_endpoints(curr_ratline);
        let mut router = Router::new(autorouter.board.layout_mut(), options.router_options);

        let this = Self {
            ratlines_iter,
            options,
            route: Some(router.route(source, target, 100.0)?),
            curr_ratline: Some(curr_ratline),
        };

        Ok(this)
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, AutorouteStatus, AutorouterError, ()>
    for AutorouteExecutionStepper
{
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
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

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
            let mut router =
                Router::new(autorouter.board.layout_mut(), self.options.router_options);

            self.curr_ratline = Some(new_ratline);
            self.route = Some(router.route(source, target, 100.0)?);
        } else {
            self.curr_ratline = None;
            //return Ok(AutorouteStatus::Finished);
        }

        Ok(AutorouteStatus::Routed(band_termseg))
    }
}

impl GetMaybeNavmesh for AutorouteExecutionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.route.as_ref().map(|route| route.navmesh())
    }
}

impl GetMaybeTrace for AutorouteExecutionStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        self.route.as_ref().map(|route| route.trace())
    }
}

impl GetGhosts for AutorouteExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.route.as_ref().map_or(&[], |route| route.ghosts())
    }
}

impl GetObstacles for AutorouteExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.route.as_ref().map_or(&[], |route| route.obstacles())
    }
}
