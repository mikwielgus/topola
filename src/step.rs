pub trait Step<C, S: TryInto<O>, E, O> {
    fn step(&mut self, context: &mut C) -> Result<S, E>;

    fn finish(&mut self, context: &mut C) -> Result<O, E> {
        loop {
            if let Ok(outcome) = self.step(context)?.try_into() {
                return Ok(outcome);
            }
        }
    }
}

pub trait StepBack<C, S: TryInto<O>, E, O> {
    fn step_back(&mut self, context: &mut C) -> Result<S, E>;
}
