use crate::{
    autorouter::{selection::Selection, Autoroute, Autorouter},
    drawing::rules::RulesTrait,
    router::RouterObserverTrait,
};

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
            Execute::Autoroute(autoroute) => autoroute.next(&mut invoker.autorouter, observer),
        }
    }
}

pub struct Invoker<R: RulesTrait> {
    autorouter: Autorouter<R>,
    history: Vec<Command>,
    undone_history: Vec<Command>,
}

impl<R: RulesTrait> Invoker<R> {
    pub fn new(autorouter: Autorouter<R>) -> Self {
        Self {
            autorouter,
            history: vec![],
            undone_history: vec![],
        }
    }

    pub fn execute(&mut self, command: Command, observer: &mut impl RouterObserverTrait<R>) {
        let mut execute = self.execute_walk(command);

        while execute.next(self, observer) {
            //
        }
    }

    pub fn execute_walk(&mut self, command: Command) -> Execute {
        let execute = match command {
            Command::Autoroute(ref selection) => {
                Execute::Autoroute(self.autorouter.autoroute_walk(selection).unwrap())
            }
        };

        self.history.push(command);
        execute
    }

    pub fn undo(&mut self) {
        let command = self.history.pop().unwrap();

        match command {
            Command::Autoroute(ref selection) => {
                self.autorouter.undo_autoroute(selection);
            }
        }

        self.undone_history.push(command);
    }

    pub fn redo(&mut self, observer: &mut impl RouterObserverTrait<R>) {
        let command = self.undone_history.pop().unwrap();
        self.execute(command, observer);
    }

    pub fn autorouter(&self) -> &Autorouter<R> {
        &self.autorouter
    }
}
