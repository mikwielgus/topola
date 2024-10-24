//! Manages the comparison of detours between two ratlines, tracking their
//! routing statuses and recording their lengths.

use std::ops::ControlFlow;

use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navcord::NavcordStepper, navmesh::Navmesh},
    stepper::Step,
};

use super::{
    autoroute::{AutorouteContinueStatus, AutorouteExecutionStepper},
    invoker::{GetGhosts, GetMaybeNavcord, GetMaybeNavmesh, GetObstacles},
    Autorouter, AutorouterError, AutorouterOptions,
};

pub struct CompareDetoursExecutionStepper {
    autoroute: AutorouteExecutionStepper,
    next_autoroute: Option<AutorouteExecutionStepper>,
    ratline1: EdgeIndex<usize>,
    ratline2: EdgeIndex<usize>,
    total_length1: f64,
    total_length2: f64,
    done: bool,
}

impl CompareDetoursExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratline1: EdgeIndex<usize>,
        ratline2: EdgeIndex<usize>,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        Ok(Self {
            autoroute: autorouter.autoroute_ratlines(vec![ratline1, ratline2], options)?,
            next_autoroute: Some(autorouter.autoroute_ratlines(vec![ratline2, ratline1], options)?),
            ratline1,
            ratline2,
            total_length1: 0.0,
            total_length2: 0.0,
            done: false,
        })
    }
}

// XXX: Do we really need this to be a stepper? We don't use at the moment, as sorting functions
// aren't steppable either. It may be useful for debugging later on tho.
impl<M: AccessMesadata> Step<Autorouter<M>, (f64, f64)> for CompareDetoursExecutionStepper {
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<(f64, f64)>, AutorouterError> {
        if self.done {
            return Ok(ControlFlow::Break((self.total_length1, self.total_length2)));
        }

        match self.autoroute.step(autorouter)? {
            ControlFlow::Continue(AutorouteContinueStatus::Running) => {
                Ok(ControlFlow::Continue(()))
            }
            ControlFlow::Continue(AutorouteContinueStatus::Routed(band_termseg)) => {
                let length = band_termseg
                    .ref_(autorouter.board.layout().drawing())
                    .length();

                if self.next_autoroute.is_some() {
                    self.total_length1 += length;
                } else {
                    self.total_length2 += length;
                }

                Ok(ControlFlow::Continue(()))
            }
            ControlFlow::Break(()) => {
                if let Some(next_autoroute) = self.next_autoroute.take() {
                    autorouter.undo_autoroute_ratlines(vec![self.ratline1, self.ratline2])?;
                    self.autoroute = next_autoroute;

                    Ok(ControlFlow::Continue(()))
                } else {
                    self.done = true;
                    autorouter.undo_autoroute_ratlines(vec![self.ratline2, self.ratline1])?;

                    Ok(ControlFlow::Break((self.total_length1, self.total_length2)))
                }
            }
        }
    }
}

impl GetMaybeNavmesh for CompareDetoursExecutionStepper {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.autoroute.maybe_navmesh()
    }
}

impl GetMaybeNavcord for CompareDetoursExecutionStepper {
    fn maybe_navcord(&self) -> Option<&NavcordStepper> {
        self.autoroute.maybe_navcord()
    }
}

impl GetGhosts for CompareDetoursExecutionStepper {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.autoroute.ghosts()
    }
}

impl GetObstacles for CompareDetoursExecutionStepper {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.autoroute.obstacles()
    }
}
