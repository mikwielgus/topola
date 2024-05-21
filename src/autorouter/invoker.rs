use crate::{
    autorouter::{history::History, selection::Selection, Autoroute, Autorouter},
    drawing::rules::RulesTrait,
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
            Execute::Autoroute(autoroute) => autoroute.next(&mut invoker.autorouter, observer),
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
        let mut execute = self.dispatch_command(command);

        while execute.next(self, &mut EmptyRouterObserver) {
            //
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

    pub fn autorouter(&self) -> &Autorouter<R> {
        &self.autorouter
    }
}
