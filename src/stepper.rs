pub trait Step<C, S: TryInto<O>, O> {
    type Error;

    fn step(&mut self, context: &mut C) -> Result<S, Self::Error>;

    fn finish(&mut self, context: &mut C) -> Result<O, Self::Error> {
        loop {
            if let Ok(outcome) = self.step(context)?.try_into() {
                return Ok(outcome);
            }
        }
    }
}

pub trait StepBack<C, S, E> {
    fn step_back(&mut self, context: &mut C) -> Result<S, E>;
}

pub trait Abort<C> {
    fn abort(&mut self, context: &mut C);
}
