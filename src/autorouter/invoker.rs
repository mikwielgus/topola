use std::cmp::Ordering;

use contracts::debug_requires;
use enum_dispatch::enum_dispatch;
use thiserror::Error;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::TraceStepper},
    step::Step,
};

use super::{
    autoroute::AutorouteCommandStepper,
    command::{Command, CommandStepper},
    compare_detours::CompareDetoursCommandStepper,
    history::{History, HistoryError},
    measure_length::MeasureLengthCommandStepper,
    place_via::PlaceViaCommandStepper,
    remove_bands::RemoveBandsCommandStepper,
    Autorouter, AutorouterError,
};

#[enum_dispatch]
pub trait GetMaybeNavmesh {
    fn maybe_navmesh(&self) -> Option<&Navmesh>;
}

#[enum_dispatch]
pub trait GetMaybeTrace {
    fn maybe_trace(&self) -> Option<&TraceStepper>;
}

#[enum_dispatch]
pub trait GetGhosts {
    fn ghosts(&self) -> &[PrimitiveShape];
}

#[enum_dispatch]
pub trait GetObstacles {
    fn obstacles(&self) -> &[PrimitiveIndex];
}

#[derive(Error, Debug, Clone)]
pub enum InvokerError {
    #[error(transparent)]
    History(#[from] HistoryError),
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

#[derive(Debug, Clone)]
pub enum InvokerStatus {
    Running,
    Finished(String),
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

    pub fn destruct(self) -> (Autorouter<M>, History, Option<Command>) {
        (self.autorouter, self.history, self.ongoing_command)
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
    pub fn execute_stepper(&mut self, command: Command) -> Result<CommandStepper, InvokerError> {
        let execute = self.dispatch_command(&command);
        self.ongoing_command = Some(command);
        execute
    }

    #[debug_requires(self.ongoing_command.is_none())]
    fn dispatch_command(&mut self, command: &Command) -> Result<CommandStepper, InvokerError> {
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

                CommandStepper::Autoroute(self.autorouter.autoroute_ratlines(ratlines, *options)?)
            }
            Command::PlaceVia(weight) => {
                CommandStepper::PlaceVia(self.autorouter.place_via(*weight)?)
            }
            Command::RemoveBands(selection) => {
                CommandStepper::RemoveBands(self.autorouter.remove_bands(selection)?)
            }
            Command::CompareDetours(selection, options) => CommandStepper::CompareDetours(
                self.autorouter.compare_detours(selection, *options)?,
            ),
            Command::MeasureLength(selection) => {
                CommandStepper::MeasureLength(self.autorouter.measure_length(selection)?)
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
        let (done, undone) = history.destruct();

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
