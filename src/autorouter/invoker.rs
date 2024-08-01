use contracts::debug_requires;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    layout::via::ViaWeight,
    router::{navmesh::Navmesh, trace::Trace},
};

use super::{
    autoroute::Autoroute,
    history::{History, HistoryError},
    place_via::PlaceVia,
    remove_bands::RemoveBands,
    selection::{BandSelection, PinSelection},
    Autorouter, AutorouterError, AutorouterStatus,
};

#[enum_dispatch]
pub trait GetMaybeNavmesh {
    fn maybe_navmesh(&self) -> Option<&Navmesh>;
}

#[enum_dispatch]
pub trait GetMaybeTrace {
    fn maybe_trace(&self) -> Option<&Trace>;
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

#[derive(Debug, Clone, Copy)]
pub enum InvokerStatus {
    Running,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Autoroute(PinSelection),
    PlaceVia(ViaWeight),
    RemoveBands(BandSelection),
}

#[enum_dispatch(GetMaybeNavmesh, GetMaybeTrace, GetGhosts, GetObstacles)]
pub enum Execute {
    Autoroute(Autoroute),
    PlaceVia(PlaceVia),
    RemoveBands(RemoveBands),
}

impl Execute {
    pub fn step<M: AccessMesadata>(
        &mut self,
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        match self.step_catch_err(invoker) {
            Ok(InvokerStatus::Running) => Ok(InvokerStatus::Running),
            Ok(InvokerStatus::Finished) => {
                if let Some(command) = invoker.ongoing_command.take() {
                    invoker.history.do_(command);
                }

                Ok(InvokerStatus::Finished)
            }
            Err(err) => {
                invoker.ongoing_command = None;
                Err(err)
            }
        }
    }

    fn step_catch_err<M: AccessMesadata>(
        &mut self,
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        match self {
            Execute::Autoroute(autoroute) => match autoroute.step(&mut invoker.autorouter)? {
                AutorouterStatus::Running => Ok(InvokerStatus::Running),
                AutorouterStatus::Routed(..) => Ok(InvokerStatus::Running),
                AutorouterStatus::Finished => Ok(InvokerStatus::Finished),
            },
            Execute::PlaceVia(place_via) => {
                place_via.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished)
            }
            Execute::RemoveBands(remove_bands) => {
                remove_bands.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished)
            }
        }
    }
}

pub struct ExecuteWithStatus {
    execute: Execute,
    maybe_status: Option<InvokerStatus>,
}

impl ExecuteWithStatus {
    pub fn new(execute: Execute) -> ExecuteWithStatus {
        Self {
            execute,
            maybe_status: None,
        }
    }

    pub fn step<M: AccessMesadata>(
        &mut self,
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        let status = self.execute.step(invoker)?;
        self.maybe_status = Some(status);
        Ok(status)
    }

    pub fn maybe_status(&self) -> Option<InvokerStatus> {
        self.maybe_status
    }
}

impl GetMaybeNavmesh for ExecuteWithStatus {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.execute.maybe_navmesh()
    }
}

impl GetMaybeTrace for ExecuteWithStatus {
    fn maybe_trace(&self) -> Option<&Trace> {
        self.execute.maybe_trace()
    }
}

impl GetGhosts for ExecuteWithStatus {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.execute.ghosts()
    }
}

impl GetObstacles for ExecuteWithStatus {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.execute.obstacles()
    }
}

pub struct Invoker<M: AccessMesadata> {
    autorouter: Autorouter<M>,
    history: History,
    ongoing_command: Option<Command>,
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
        let mut execute = self.execute_walk(command)?;

        loop {
            let status = match execute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished = status {
                self.history.set_undone(std::iter::empty());
                return Ok(());
            }
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    pub fn execute_walk(&mut self, command: Command) -> Result<Execute, InvokerError> {
        let execute = self.dispatch_command(&command);
        self.ongoing_command = Some(command);
        execute
    }

    #[debug_requires(self.ongoing_command.is_none())]
    fn dispatch_command(&mut self, command: &Command) -> Result<Execute, InvokerError> {
        match command {
            Command::Autoroute(selection) => Ok::<Execute, InvokerError>(Execute::Autoroute(
                self.autorouter.autoroute_walk(selection)?,
            )),
            Command::PlaceVia(weight) => Ok::<Execute, InvokerError>(Execute::PlaceVia(
                self.autorouter.place_via_walk(*weight)?,
            )),
            Command::RemoveBands(selection) => Ok::<Execute, InvokerError>(Execute::RemoveBands(
                self.autorouter.remove_bands_walk(selection)?,
            )),
        }
    }

    #[debug_requires(self.ongoing_command.is_none())]
    pub fn undo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_done()?;

        match command {
            Command::Autoroute(ref selection) => self.autorouter.undo_autoroute(selection),
            Command::PlaceVia(weight) => self.autorouter.undo_place_via(*weight),
            Command::RemoveBands(ref selection) => self.autorouter.undo_remove_bands(selection),
        }

        Ok::<(), InvokerError>(self.history.undo()?)
    }

    //#[debug_requires(self.ongoing.is_none())]
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_undone()?.clone();
        let mut execute = self.execute_walk(command)?;

        loop {
            let status = match execute.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished = status {
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
