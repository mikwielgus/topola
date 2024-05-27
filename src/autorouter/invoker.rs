use core::fmt;

use crate::{
    autorouter::{history::History, selection::Selection, Autoroute, Autorouter},
    drawing::rules::RulesTrait,
    layout::Layout,
    router::{EmptyRouterObserver, RouterObserverTrait},
};

#[derive(serde::Serialize, serde::Deserialize)]
pub enum Command {
    Autoroute(Selection),
}

pub enum Execute {
    Autoroute(Autoroute),
}

impl Execute {
    pub fn next<R: RulesTrait>(
        &mut self,
        invoker: &mut Invoker<R>,
        observer: &mut impl RouterObserverTrait<R>,
    ) -> bool {
        match self {
            Execute::Autoroute(autoroute) => autoroute.step(&mut invoker.autorouter, observer),
        }
    }
}

pub struct Invoker<R: RulesTrait> {
    autorouter: Autorouter<R>,
    history: History,
}

impl<R: RulesTrait> Invoker<R> {
    pub fn new(autorouter: Autorouter<R>) -> Self {
        Self {
            autorouter,
            history: History::new(),
        }
    }

    pub fn execute(&mut self, command: Command, observer: &mut impl RouterObserverTrait<R>) {
        let mut execute = self.execute_walk(command);

        while execute.next(self, observer) {
            //
        }

        self.history.set_undone(std::iter::empty());
    }

    pub fn execute_walk(&mut self, command: Command) -> Execute {
        let execute = self.dispatch_command(&command);
        self.history.do_(command);
        execute
    }

    fn dispatch_command(&self, command: &Command) -> Execute {
        match command {
            Command::Autoroute(ref selection) => {
                Execute::Autoroute(self.autorouter.autoroute_walk(selection).unwrap())
            }
        }
    }

    pub fn undo(&mut self) {
        let command = self.history.done().last().unwrap();

        match command {
            Command::Autoroute(ref selection) => self.autorouter.undo_autoroute(selection),
        }

        self.history.undo();
    }

    pub fn redo(&mut self) {
        let command = self.history.undone().last().unwrap();
        let mut execute = self.dispatch_command(command);

        while execute.next(self, &mut EmptyRouterObserver) {
            //
        }

        self.history.redo();
    }

    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.destructure();

        for command in done {
            self.execute(command, &mut EmptyRouterObserver);
        }

        self.history.set_undone(undone.into_iter());
    }

    pub fn autorouter(&self) -> &Autorouter<R> {
        &self.autorouter
    }

    pub fn history(&self) -> &History {
        &self.history
    }
}
