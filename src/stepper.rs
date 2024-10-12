use std::ops::ControlFlow;

pub trait Step<Ctx, B, C = ()> {
    type Error;

    fn step(&mut self, context: &mut Ctx) -> Result<ControlFlow<B, C>, Self::Error>;

    fn finish(&mut self, context: &mut Ctx) -> Result<B, Self::Error> {
        loop {
            if let ControlFlow::Break(outcome) = self.step(context)? {
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
