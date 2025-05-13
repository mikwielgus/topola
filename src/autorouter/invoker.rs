// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages the execution of routing commands within the autorouting system.

use std::{cmp::Ordering, ops::ControlFlow};

use contracts_try::debug_requires;
use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use thiserror::Error;

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{edit::ApplyGeometryEdit, primitive::PrimitiveShape},
    router::{
        astar::AstarStepper,
        navcord::Navcord,
        navmesh::{Navmesh, NavvertexIndex},
    },
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

/// Trait for getting the A* stepper to display its data on the debug overlay,
/// most importantly the navmesh which is owned by the A* stepper.
#[enum_dispatch]
pub trait GetMaybeAstarStepper {
    fn maybe_astar(&self) -> Option<&AstarStepper<Navmesh, f64>>;
}

/// Trait for getting the navcord to display it on the debug overlay.
#[enum_dispatch]
pub trait GetMaybeNavcord {
    fn maybe_navcord(&self) -> Option<&Navcord>;
}

/// Trait for getting ghosts to display on the debug overlay. Ghosts are the
/// shapes that Topola attempted to create but failed due to them infringing on
/// other shapes.
#[enum_dispatch]
pub trait GetGhosts {
    fn ghosts(&self) -> &[PrimitiveShape];
}

/// Trait for getting the obstacles that prevented Topola from creating
/// new objects (the shapes of these objects can be obtained with the above
/// `GetGhosts` trait), for the purpose of displaying these obstacles on the
/// debug overlay.
#[enum_dispatch]
pub trait GetObstacles {
    fn obstacles(&self) -> &[PrimitiveIndex];
}

#[enum_dispatch]
/// Trait for getting text strings with debug information attached to navmesh
/// edges and vertices.
pub trait GetNavmeshDebugTexts {
    fn navvertex_debug_text(&self, navvertex: NavvertexIndex) -> Option<&str>;
    fn navedge_debug_text(&self, navedge: (NavvertexIndex, NavvertexIndex)) -> Option<&str>;
}

/// Error types that can occur during the invocation of commands.
#[derive(Error, Debug, Clone)]
pub enum InvokerError {
    /// Wraps errors related to command history operations.
    #[error(transparent)]
    History(#[from] HistoryError),

    /// Wraps errors related to autorouter operations.
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

#[derive(Getters, Dissolve)]
pub struct Invoker<M> {
    /// Instance for executing desired autorouting commands.
    pub(super) autorouter: Autorouter<M>,
    /// History of executed commands.
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
    /// Executes a command, managing the command status and history.
    ///
    /// This function is used to pass the [`Command`] to [`Invoker::execute_stepper`]
    /// function, and control its execution status.
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
    /// Pass given command to be executed.
    ///
    /// Function used to set given [`Command`] to ongoing state, dispatch and execute it.
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
    /// Undo last command.
    pub fn undo(&mut self) -> Result<(), InvokerError> {
        let last_done = self.history.last_done()?;

        if let Some(edit) = last_done.edit() {
            self.autorouter.board.apply(&edit.reverse());
        }

        Ok(self.history.undo()?)
    }

    //#[debug_requires(self.ongoing_command.is_none())]
    /// Redo last command.
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let last_undone = self.history.last_undone()?;

        if let Some(edit) = last_undone.edit() {
            self.autorouter.board.apply(edit);
        }

        Ok(self.history.redo()?)
    }

    #[debug_requires(self.ongoing_command.is_none())]
    /// Replay last command.
    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.dissolve();

        for entry in done {
            self.execute(entry.command().clone());
        }

        self.history.set_undone(undone);
    }
}
