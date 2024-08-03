use contracts::{debug_ensures, debug_requires};
use thiserror::Error;

use crate::{
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, rules::AccessRules},
    layout::Layout,
    step::{IsFinished, Step, StepBack},
};

use super::{
    draw::{Draw, DrawException},
    navmesh::{Navmesh, NavvertexIndex},
    trace::{Trace, TraceStepInput},
};

#[derive(Error, Debug, Clone, Copy)]
pub enum TracerException {
    #[error(transparent)]
    CannotDraw(#[from] DrawException),
    #[error("cannot wrap")]
    CannotWrap,
}

pub enum TracerStatus {
    Running,
    Finished,
}

impl IsFinished for TracerStatus {
    fn finished(&self) -> bool {
        matches!(self, TracerStatus::Finished)
    }
}

#[derive(Debug)]
pub struct Tracer<'a, R: AccessRules> {
    pub layout: &'a mut Layout<R>,
}

impl<'a, R: AccessRules> Tracer<'a, R> {
    pub fn new(layout: &mut Layout<R>) -> Tracer<R> {
        Tracer { layout }
    }

    pub fn start(
        &mut self,
        source: FixedDotIndex,
        source_navvertex: NavvertexIndex,
        width: f64,
    ) -> Trace {
        Trace::new(source, source_navvertex, width)
    }

    pub fn finish(
        &mut self,
        _navmesh: &Navmesh,
        trace: &mut Trace,
        target: FixedDotIndex,
        width: f64,
    ) -> Result<BandTermsegIndex, TracerException> {
        Ok(Draw::new(self.layout).finish_in_dot(trace.head, target, width)?)
    }

    #[debug_requires(path[0] == trace.path[0])]
    #[debug_ensures(ret.is_ok() -> trace.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        navmesh: &Navmesh,
        trace: &mut Trace,
        path: &[NavvertexIndex],
        width: f64,
    ) -> Result<(), TracerException> {
        let prefix_length = trace
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = trace.path.len();
        self.undo_path(trace, length - prefix_length);
        Ok::<(), TracerException>(self.path(navmesh, trace, &path[prefix_length..], width)?)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + path.len()))]
    pub fn path(
        &mut self,
        navmesh: &Navmesh,
        trace: &mut Trace,
        path: &[NavvertexIndex],
        width: f64,
    ) -> Result<(), TracerException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = trace.step(&mut TraceStepInput {
                tracer: self,
                navmesh,
                to: *vertex,
                width,
            }) {
                self.undo_path(trace, i);
                return Err(err.into());
            }
        }

        Ok(())
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - step_count))]
    pub fn undo_path(&mut self, trace: &mut Trace, step_count: usize) {
        for _ in 0..step_count {
            let _ = trace.step_back(self);
        }
    }
}
