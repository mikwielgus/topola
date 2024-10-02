//! Manages autorouting of ratlines in a layout, tracking status and processed
//!  routing steps. Provides access to navigation meshes, traces, ghost shapes,
//! and obstacles encountered during routing.

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

/// Represents the current status of the autoroute operation.
pub enum AutorouteStatus {
    /// The autoroute is currently running and in progress.
    Running,
    /// A specific segment has been successfully routed.
    Routed(BandTermsegIndex),
    /// The autoroute process has completed successfully.
    Finished,
}

impl TryInto<()> for AutorouteStatus {
    type Error = ();
    /// Attempts to get the [`Result`] from the  [`AutorouteStatus`].
    ///
    /// This implementation allows transitioning from [`AutorouteStatus`] to a 
    /// [`Result`]. It returns success for the  [`AutorouteStatus::Finished`] state
    /// or an error for [`AutorouteStatus::Running`] or [`AutorouteStatus::Routed`] states.
    fn try_into(self) -> Result<(), ()> {
        match self {
            AutorouteStatus::Running => Err(()),
            AutorouteStatus::Routed(..) => Err(()),
            AutorouteStatus::Finished => Ok(()),
        }
    }
}

/// Manages the autorouting process across multiple ratlines.
pub struct AutorouteExecutionStepper {
    /// An iterator over ratlines that tracks which segments still need to be routed.
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    /// The options for the autorouting process, defining how routing should be carried out.
    options: AutorouterOptions,
    /// Stores the current route being processed, if any.
    route: Option<RouteStepper>,
    /// Keeps track of the current ratline being routed, if one is active.
    curr_ratline: Option<EdgeIndex<usize>>,
}

impl AutorouteExecutionStepper {
    /// Initializes a new [`AutorouteExecutionStepper`] instance.
    ///
    /// This method sets up the routing process by accepting the execution properties.
    /// It prepares the first ratline to route 
    /// and stores the associated data for future routing steps.
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
    /// Retrieves an optional reference to the navigation mesh from the current route.
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.route.as_ref().map(|route| route.navmesh())
    }
}

impl GetMaybeTrace for AutorouteExecutionStepper {
    /// Retrieves an optional reference to the trace from the current route.
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        self.route.as_ref().map(|route| route.trace())
    }
}

impl GetGhosts for AutorouteExecutionStepper {
    /// Retrieves ghost shapes from the current route.
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.route.as_ref().map_or(&[], |route| route.ghosts())
    }
}

impl GetObstacles for AutorouteExecutionStepper {
    /// Retrieves obstacles encountered during routing.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.route.as_ref().map_or(&[], |route| route.obstacles())
    }
}
