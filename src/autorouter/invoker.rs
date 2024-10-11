//! Manages the execution of routing commands within the autorouting system.

use std::cmp::Ordering;

use contracts_try::debug_requires;
use derive_getters::Dissolve;
use enum_dispatch::enum_dispatch;
use thiserror::Error;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navcord::NavcordStepper, navmesh::Navmesh},
    stepper::Step,
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    execution::{Command, ExecutionStepper},
    history::{History, HistoryError},
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    remove_bands::RemoveBandsExecutionStepper,
    Autorouter, AutorouterError,
};

#[enum_dispatch]
pub trait GetMaybeNavmesh {
    fn maybe_navmesh(&self) -> Option<&Navmesh>;
}

#[enum_dispatch]
pub trait GetMaybeNavcord {
    fn maybe_navcord(&self) -> Option<&NavcordStepper>;
}

#[enum_dispatch]
pub trait GetGhosts {
    fn ghosts(&self) -> &[PrimitiveShape];
}

#[enum_dispatch]
pub trait GetObstacles {
    fn obstacles(&self) -> &[PrimitiveIndex];
}

#[derive(Debug, Clone)]
pub enum InvokerStatus {
    Running,
    Finished(String),
}

#[derive(Error, Debug, Clone)]
pub enum InvokerError {
    #[error(transparent)]
    History(#[from] HistoryError),
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

impl TryInto<()> for InvokerStatus {
    type Error = ();
    fn try_into(self) -> Result<(), ()> {
        match self {
            InvokerStatus::Running => Err(()),
            InvokerStatus::Finished(..) => Ok(()),
        }
    }
}

#[derive(Dissolve)]
pub struct Invoker<M: AccessMesadata> {
    pub(super) autorouter: Autorouter<M>,
    pub(super) history: History,
    pub(super) ongoing_command: Option<Command>,
}

impl<M: AccessMesadata> Invoker<M> {
    pub fn new(autorouter: Autorouter<M>) -> Self {
        Self::new_with_history(autorouter, History::new())
    }

    pub fn new_with_history(autorouter: Autorouter<M>, history: History) -> Self {
        Self {
            autorouter,
            history,
            ongoing_command: None,
        }
    }

    //#[debug_requires(self.ongoing_command.is_none())]
    pub fn execute(&mut self, command: Command) -> Result<(), InvokerError> {
        let mut execute = self.execute_stepper(command)?;

        loop {
            let status = match execute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished(..) = status {
                self.history.set_undone(std::iter::empty());
                return Ok(());
            }
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    pub fn execute_stepper(&mut self, command: Command) -> Result<ExecutionStepper, InvokerError> {
        let execute = self.dispatch_command(&command);
        self.ongoing_command = Some(command);
        execute
    }

    #[debug_requires(self.ongoing_command.is_none())]
    fn dispatch_command(&mut self, command: &Command) -> Result<ExecutionStepper, InvokerError> {
        Ok(match command {
            Command::Autoroute(selection, options) => {
                let mut ratlines = self.autorouter.selected_ratlines(selection);

                if options.presort_by_pairwise_detours {
                    ratlines.sort_unstable_by(|a, b| {
                        let mut compare_detours = self
                            .autorouter
                            .compare_detours_ratlines(*a, *b, *options)
                            .unwrap();
                        if let Ok((al, bl)) = compare_detours.finish(&mut self.autorouter) {
                            PartialOrd::partial_cmp(&al, &bl).unwrap()
                        } else {
                            Ordering::Equal
                        }
                    });
                }

                ExecutionStepper::Autoroute(self.autorouter.autoroute_ratlines(ratlines, *options)?)
            }
            Command::PlaceVia(weight) => {
                ExecutionStepper::PlaceVia(self.autorouter.place_via(*weight)?)
            }
            Command::RemoveBands(selection) => {
                ExecutionStepper::RemoveBands(self.autorouter.remove_bands(selection)?)
            }
            Command::CompareDetours(selection, options) => ExecutionStepper::CompareDetours(
                self.autorouter.compare_detours(selection, *options)?,
            ),
            Command::MeasureLength(selection) => {
                ExecutionStepper::MeasureLength(self.autorouter.measure_length(selection)?)
            }
        })
    }

    #[debug_requires(self.ongoing_command.is_none())]
    pub fn undo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_done()?;

        match command {
            Command::Autoroute(ref selection, ..) => {
                self.autorouter.undo_autoroute(selection)?;
            }
            Command::PlaceVia(weight) => {
                self.autorouter.undo_place_via(*weight);
            }
            Command::RemoveBands(ref selection) => {
                self.autorouter.undo_remove_bands(selection);
            }
            Command::CompareDetours(..) => {}
            Command::MeasureLength(..) => {}
        }

        Ok(self.history.undo()?)
    }

    //#[debug_requires(self.ongoing.is_none())]
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_undone()?.clone();
        let mut execute = self.execute_stepper(command)?;

        loop {
            let status = match execute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished(..) = status {
                return Ok(self.history.redo()?);
            }
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.dissolve();

        for command in done {
            self.execute(command);
        }

        self.history.set_undone(undone.into_iter());
    }

    pub fn autorouter(&self) -> &Autorouter<M> {
        &self.autorouter
    }

    pub fn history(&self) -> &History {
        &self.history
    }
}
