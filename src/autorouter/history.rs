use serde::{Deserialize, Serialize};

use crate::autorouter::invoker::Command;

#[derive(Serialize, Deserialize)]
pub struct History {
    done: Vec<Command>,
    undone: Vec<Command>,
}

impl History {
    pub fn new() -> Self {
        Self {
            done: vec![],
            undone: vec![],
        }
    }

    pub fn do_(&mut self, command: Command) {
        self.done.push(command);
    }

    pub fn undo(&mut self) {
        let command = self.done.pop().unwrap();
        self.undone.push(command);
    }

    pub fn redo(&mut self) {
        let command = self.undone.pop().unwrap();
        self.done.push(command);
    }

    pub fn done(&self) -> &[Command] {
        &self.done
    }

    pub fn undone(&self) -> &[Command] {
        &self.undone
    }
}
