use std::sync::{Arc, Mutex};

use contracts::debug_ensures;

use crate::{
    drawing::{
        bend::LooseBendIndex,
        dot::FixedDotIndex,
        guide::{BareHead, Head, SegbendHead},
        rules::RulesTrait,
    },
    layout::Layout,
    router::{
        draw::{Draw, DrawException},
        navmesh::{Navmesh, VertexIndex},
    },
};

#[derive(Debug)]
pub struct Trace {
    pub path: Vec<VertexIndex>,
    pub head: Head,
    pub width: f64,
}

#[derive(Debug)]
pub struct Tracer<R: RulesTrait> {
    pub layout: Arc<Mutex<Layout<R>>>,
}

impl<R: RulesTrait> Tracer<R> {
    pub fn new(layout: Arc<Mutex<Layout<R>>>) -> Self {
        Tracer { layout }
    }

    pub fn start(&mut self, from: FixedDotIndex, width: f64) -> Trace {
        Trace {
            path: vec![from.into()],
            head: BareHead { dot: from }.into(),
            width,
        }
    }

    pub fn finish(
        &mut self,
        trace: &mut Trace,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        Draw::new(&mut self.layout.lock().unwrap()).finish_in_dot(trace.head, into, width)?;
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        trace: &mut Trace,
        path: &[VertexIndex],
        width: f64,
    ) -> Result<(), DrawException> {
        let prefix_length = trace
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = trace.path.len();
        self.undo_path(trace, length - prefix_length);
        self.path(trace, &path[prefix_length..], width)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + path.len()))]
    pub fn path(
        &mut self,
        trace: &mut Trace,
        path: &[VertexIndex],
        width: f64,
    ) -> Result<(), DrawException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = self.step(trace, *vertex, width) {
                self.undo_path(trace, i);
                return Err(err);
            }
        }

        Ok(())
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - step_count))]
    pub fn undo_path(&mut self, trace: &mut Trace, step_count: usize) {
        for _ in 0..step_count {
            self.undo_step(trace);
        }
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn step(
        &mut self,
        trace: &mut Trace,
        to: VertexIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        trace.head = self.wrap(trace.head, to, width)?.into();
        trace.path.push(to);

        Ok::<(), DrawException>(())
    }

    fn wrap(
        &mut self,
        head: Head,
        around: VertexIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        match around {
            VertexIndex::FixedDot(dot) => self.wrap_around_fixed_dot(head, dot, width),
            VertexIndex::FixedBend(_fixed_bend) => todo!(),
            VertexIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(head, loose_bend, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        let head = Draw::new(&mut self.layout.lock().unwrap()).segbend_around_dot(
            head,
            around.into(),
            width,
        )?;
        Ok(head)
    }

    fn wrap_around_loose_bend(
        &mut self,
        head: Head,
        around: LooseBendIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        let head = Draw::new(&mut self.layout.lock().unwrap()).segbend_around_bend(
            head,
            around.into(),
            width,
        )?;

        Ok(head)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, trace: &mut Trace) {
        if let Head::Segbend(head) = trace.head {
            trace.head = Draw::new(&mut self.layout.lock().unwrap())
                .undo_segbend(head)
                .unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }
}
