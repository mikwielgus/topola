pub trait IsFinished {
    fn finished(&self) -> bool;
}

pub trait Step<I, S: IsFinished, E> {
    fn step(&mut self, input: &mut I) -> Result<S, E>;
}

pub trait StepBack<I, S: IsFinished, E> {
    fn step_back(&mut self, input: &mut I) -> Result<S, E>;
}
