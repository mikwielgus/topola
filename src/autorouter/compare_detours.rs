use petgraph::graph::EdgeIndex;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{primitive::PrimitiveShape, shape::MeasureLength},
    graph::MakeRef,
    router::{navmesh::Navmesh, trace::Trace},
    step::Step,
};

use super::{
    autoroute::{Autoroute, AutorouteStatus},
    invoker::{GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles},
    Autorouter, AutorouterError,
};

pub enum CompareDetoursStatus {
    Running,
    Finished(f64),
}

impl TryInto<f64> for CompareDetoursStatus {
    type Error = ();
    fn try_into(self) -> Result<f64, ()> {
        match self {
            CompareDetoursStatus::Running => Err(()),
            CompareDetoursStatus::Finished(delta) => Ok(delta),
        }
    }
}

pub struct CompareDetours {
    autoroute: Autoroute,
    next_autoroute: Option<Autoroute>,
    ratline1: EdgeIndex<usize>,
    ratline2: EdgeIndex<usize>,
    total_length1: Option<f64>,
    total_length2: Option<f64>,
    done: bool,
}

impl CompareDetours {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        ratline1: EdgeIndex<usize>,
        ratline2: EdgeIndex<usize>,
    ) -> Result<Self, AutorouterError> {
        Ok(Self {
            autoroute: autorouter.autoroute_ratlines(vec![ratline1, ratline2])?,
            next_autoroute: Some(autorouter.autoroute_ratlines(vec![ratline2, ratline1])?),
            ratline1,
            ratline2,
            total_length1: None,
            total_length2: None,
            done: false,
        })
    }
}

// XXX: Do we really need this to be a stepper? We don't use at the moment, as sorting functions
// aren't steppable either. It may be useful for debugging later on tho.
impl<M: AccessMesadata> Step<Autorouter<M>, CompareDetoursStatus, AutorouterError, f64>
    for CompareDetours
{
    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<CompareDetoursStatus, AutorouterError> {
        if self.done {
            return Ok(CompareDetoursStatus::Finished(
                self.total_length1.unwrap() - self.total_length2.unwrap(),
            ));
        }

        match self.autoroute.step(autorouter)? {
            AutorouteStatus::Running => Ok(CompareDetoursStatus::Running),
            AutorouteStatus::Routed(band_termseg) => {
                let length = band_termseg
                    .ref_(autorouter.board.layout().drawing())
                    .length();

                if self.total_length1.is_none() {
                    self.total_length1 = Some(length);
                } else if self.total_length2.is_none() {
                    self.total_length2 = Some(length);
                } else {
                    panic!();
                }

                Ok(CompareDetoursStatus::Running)
            }
            AutorouteStatus::Finished => {
                if let Some(next_autoroute) = self.next_autoroute.take() {
                    autorouter.undo_autoroute_ratlines(vec![self.ratline1, self.ratline2]);
                    self.autoroute = next_autoroute;

                    Ok(CompareDetoursStatus::Running)
                } else {
                    self.done = true;
                    autorouter.undo_autoroute_ratlines(vec![self.ratline2, self.ratline1]);

                    Ok(CompareDetoursStatus::Finished(
                        self.total_length1.unwrap() - self.total_length2.unwrap(),
                    ))
                }
            }
        }
    }
}

impl GetMaybeNavmesh for CompareDetours {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.autoroute.maybe_navmesh()
    }
}

impl GetMaybeTrace for CompareDetours {
    fn maybe_trace(&self) -> Option<&Trace> {
        self.autoroute.maybe_trace()
    }
}

impl GetGhosts for CompareDetours {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.autoroute.ghosts()
    }
}

impl GetObstacles for CompareDetours {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.autoroute.obstacles()
    }
}
