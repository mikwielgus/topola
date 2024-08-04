pub trait GetMaybeOutcome<O> {
    fn maybe_outcome(&self) -> Option<O>;
}

pub trait Step<C, S: GetMaybeOutcome<O>, E, O> {
    fn step(&mut self, context: &mut C) -> Result<S, E>;

    fn finish(&mut self, context: &mut C) -> Result<O, E> {
        loop {
            if let Some(outcome) = self.step(context)?.maybe_outcome() {
                return Ok(outcome);
            }
        }
    }
}

pub trait StepBack<C, S: GetMaybeOutcome<O>, E, O> {
    fn step_back(&mut self, context: &mut C) -> Result<S, E>;
}
