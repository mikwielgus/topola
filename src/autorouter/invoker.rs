use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    autorouter::{
        autoroute::Autoroute,
        history::{History, HistoryError},
        place_via::PlaceVia,
        selection::Selection,
        Autorouter, AutorouterError, AutorouterStatus,
    },
    board::mesadata::MesadataTrait,
    layout::via::ViaWeight,
    router::{EmptyRouterObserver, RouterObserverTrait},
};

#[derive(Error, Debug, Clone)]
pub enum InvokerError {
    #[error(transparent)]
    History(#[from] HistoryError),
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

pub enum InvokerStatus {
    Running,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Autoroute(Selection),
    PlaceVia(ViaWeight),
}

pub enum Execute {
    Autoroute(Autoroute),
    PlaceVia(PlaceVia),
}

impl Execute {
    pub fn step<M: MesadataTrait>(
        &mut self,
        invoker: &mut Invoker<M>,
        observer: &mut impl RouterObserverTrait<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        match self {
            Execute::Autoroute(autoroute) => {
                match autoroute.step(&mut invoker.autorouter, observer)? {
                    AutorouterStatus::Running => Ok(InvokerStatus::Running),
                    AutorouterStatus::Finished => Ok(InvokerStatus::Finished),
                }
            }
            Execute::PlaceVia(place_via) => {
                place_via.doit(&mut invoker.autorouter)?;
                Ok(InvokerStatus::Finished)
            }
        }
    }
}

pub struct Invoker<M: MesadataTrait> {
    autorouter: Autorouter<M>,
    history: History,
}

impl<M: MesadataTrait> Invoker<M> {
    pub fn new(autorouter: Autorouter<M>) -> Self {
        Self::new_with_history(autorouter, History::new())
    }

    pub fn new_with_history(autorouter: Autorouter<M>, history: History) -> Self {
        Self {
            autorouter,
            history,
        }
    }

    pub fn destruct(self) -> (Autorouter<M>, History) {
        (self.autorouter, self.history)
    }

    pub fn execute(
        &mut self,
        command: Command,
        observer: &mut impl RouterObserverTrait<M>,
    ) -> Result<(), InvokerError> {
        let mut execute = self.execute_walk(command);

        loop {
            let status = match execute.step(self, observer) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished = status {
                self.history.set_undone(std::iter::empty());
                return Ok(());
            }
        }
    }

    pub fn execute_walk(&mut self, command: Command) -> Execute {
        let execute = self.dispatch_command(&command);
        self.history.do_(command);
        execute
    }

    fn dispatch_command(&mut self, command: &Command) -> Execute {
        match command {
            Command::Autoroute(selection) => {
                Execute::Autoroute(self.autorouter.autoroute_walk(selection).unwrap())
            }
            Command::PlaceVia(weight) => {
                Execute::PlaceVia(self.autorouter.place_via_walk(*weight).unwrap())
            }
        }
    }

    pub fn undo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_done()?;

        match command {
            Command::Autoroute(ref selection) => self.autorouter.undo_autoroute(selection),
            Command::PlaceVia(weight) => self.autorouter.undo_place_via(*weight),
        }

        Ok(self.history.undo()?)
    }

    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let command = self.history.last_undone()?.clone();
        let mut execute = self.dispatch_command(&command);

        loop {
            let status = match execute.step(self, &mut EmptyRouterObserver) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let InvokerStatus::Finished = status {
                return Ok(self.history.redo()?);
            }
        }
    }

    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.destructure();

        for command in done {
            self.execute(command, &mut EmptyRouterObserver);
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
