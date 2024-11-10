//! Manages the execution of routing commands within the autorouting system.

use std::{cmp::Ordering, ops::ControlFlow};

use contracts_try::debug_requires;
use derive_getters::{Dissolve, Getters};
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
/// Getter trait to obtain Navigation Mesh
///
/// Navigation Mesh is possible routes between
/// two points
pub trait GetMaybeNavmesh {
    /// Returns Navigation Mesh if possible
    fn maybe_navmesh(&self) -> Option<&Navmesh>;
}

#[enum_dispatch]
/// Getter for Navigation Cord
///
/// Navigation Cord is the possible path of
/// ongoing autorouting process
pub trait GetMaybeNavcord {
    /// Gets the Navigation Cord if possible
    fn maybe_navcord(&self) -> Option<&NavcordStepper>;
}

#[enum_dispatch]
/// Requires Ghosts implementations
///
/// Ghosts are possible shapes of routing
/// bands
pub trait GetGhosts {
    /// Retrieves the ghosts associated with the execution.
    fn ghosts(&self) -> &[PrimitiveShape];
}

#[enum_dispatch]
/// Getter for the Obstacles
///
/// Obstacles are shapes of existing bands
/// to be avoided by the new band
pub trait GetObstacles {
    /// Returns possible Obstacles
    fn obstacles(&self) -> &[PrimitiveIndex];
}

/// Error types that can occur during the invocation of commands
#[derive(Error, Debug, Clone)]
pub enum InvokerError {    
    /// Wraps errors related to command history operations
    #[error(transparent)]
    History(#[from] HistoryError),
    
    /// Wraps errors related to autorouter operations
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

#[derive(Getters, Dissolve)]
/// Structure that manages the execution and history of commands within the autorouting system
pub struct Invoker<M: AccessMesadata> {
    /// Instance for executing desired autorouting commands
    pub(super) autorouter: Autorouter<M>,
    /// History of executed commands
    pub(super) history: History,
    /// Currently ongoing command type.
    pub(super) ongoing_command: Option<Command>,
}

impl<M: AccessMesadata> Invoker<M> {
    /// Creates a new instance of Invoker with the given autorouter instance
    pub fn new(autorouter: Autorouter<M>) -> Self {
        Self::new_with_history(autorouter, History::new())
    }

    /// Creates a new instance of Invoker with the given autorouter and history
    pub fn new_with_history(autorouter: Autorouter<M>, history: History) -> Self {
        Self {
            autorouter,
            history,
            ongoing_command: None,
        }
    }

    //#[debug_requires(self.ongoing_command.is_none())]
    /// Executes a command, managing the command status and history
    ///
    /// This function is used to pass the [`Command`] to [`Invoker::execute_stepper`]
    /// function, and control its execution status
    pub fn execute(&mut self, command: Command) -> Result<(), InvokerError> {
        let mut execute = self.execute_stepper(command)?;

        loop {
            let status = execute.step(self)?;

            if let ControlFlow::Break(..) = status {
                self.history.set_undone(std::iter::empty());
                return Ok(());
            }
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    /// Pass given command to be executed
    ///
    /// Function used to set given [`Command`] to ongoing state, dispatch and execute it
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
    /// Undo last command
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

    //#[debug_requires(self.ongoing_command.is_none())]
    /// Redo last command
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_undone()?.clone();
        let mut execute = self.execute_stepper(command)?;

        loop {
            let status = match execute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let ControlFlow::Break(..) = status {
                return Ok(self.history.redo()?);
            }
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    /// Replay last command
    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.dissolve();

        for command in done {
            self.execute(command);
        }

        self.history.set_undone(undone.into_iter());
    }
}
