pub trait IsFinished {
    fn finished(&self) -> bool;
}

pub trait Step<C, S: IsFinished, E> {
    fn step(&mut self, context: &mut C) -> Result<S, E>;
}
