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
}

impl<R: RulesTrait> Invoker<R> {
    pub fn new(autorouter: Autorouter<R>) -> Self {
        Self { autorouter }
    }

    pub fn execute(&mut self, command: &Command, observer: &mut impl RouterObserverTrait<R>) {
        let mut execute = self.execute_walk(command);

        while execute.next(self, observer) {
            //
        }
    }

    pub fn execute_walk(&mut self, command: &Command) -> Execute {
        match command {
            Command::Autoroute(selection) => {
                Execute::Autoroute(self.autorouter.autoroute_walk(&selection).unwrap())
            }
        }
    }

    pub fn undo(&mut self) {
        todo!();
    }

    pub fn redo(&mut self) {
        todo!();
    }

    pub fn autorouter(&self) -> &Autorouter<R> {
        &self.autorouter
    }
}
