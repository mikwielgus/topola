//! Manages the comparison of detours between two ratlines, tracking their
//! routing statuses and lengths. Facilitates stepwise processing of routing
//! while providing access to navigation meshes, traces, ghost shapes, and
//! obstacles encountered.

use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navmesh::Navmesh, trace::TraceStepper},
    stepper::Step,
};

use super::{
    autoroute::{AutorouteExecutionStepper, AutorouteStatus},
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError, AutorouterOptions,
};

pub enum CompareDetoursStatus {
    Running,
    Finished(f64, f64),
}

impl TryInto<(f64, f64)> for CompareDetoursStatus {
    type Error = ();
    fn try_into(self) -> Result<(f64, f64), ()> {
        match self {
            CompareDetoursStatus::Running => Err(()),
            CompareDetoursStatus::Finished(total_length1, total_length2) => {
                Ok((total_length1, total_length2))
            }
        }
    }
}

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
impl<M: AccessMesadata> Step<Autorouter<M>, CompareDetoursStatus, AutorouterError, (f64, f64)>
    for CompareDetoursExecutionStepper
{
    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<CompareDetoursStatus, AutorouterError> {
        if self.done {
            return Ok(CompareDetoursStatus::Finished(
                self.total_length1,
                self.total_length2,
            ));
        }

        match self.autoroute.step(autorouter)? {
            AutorouteStatus::Running => Ok(CompareDetoursStatus::Running),
            AutorouteStatus::Routed(band_termseg) => {
                let length = band_termseg
                    .ref_(autorouter.board.layout().drawing())
                    .length();

                if self.next_autoroute.is_some() {
                    self.total_length1 += length;
                } else {
                    self.total_length2 += length;
                }

                Ok(CompareDetoursStatus::Running)
            }
            AutorouteStatus::Finished => {
                if let Some(next_autoroute) = self.next_autoroute.take() {
                    autorouter.undo_autoroute_ratlines(vec![self.ratline1, self.ratline2])?;
                    self.autoroute = next_autoroute;

                    Ok(CompareDetoursStatus::Running)
                } else {
                    self.done = true;
                    autorouter.undo_autoroute_ratlines(vec![self.ratline2, self.ratline1])?;

                    Ok(CompareDetoursStatus::Finished(
                        self.total_length1,
                        self.total_length2,
                    ))
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

impl GetMaybeTrace for CompareDetoursExecutionStepper {
    fn maybe_trace(&self) -> Option<&TraceStepper> {
        self.autoroute.maybe_trace()
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
