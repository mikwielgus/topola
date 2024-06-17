use contracts::debug_ensures;

use crate::{
    drawing::{
        band::BandIndex,
        bend::LooseBendIndex,
        dot::FixedDotIndex,
        guide::{BareHead, CaneHead, Head},
        rules::RulesTrait,
    },
    layout::Layout,
    router::{
        draw::{Draw, DrawException},
        navmesh::NavvertexIndex,
    },
};

#[derive(Debug)]
pub struct Trace {
    pub path: Vec<NavvertexIndex>,
    pub head: Head,
    pub width: f64,
}

#[derive(Debug)]
pub struct Tracer<'a, R: RulesTrait> {
    pub layout: &'a mut Layout<R>,
}

impl<'a, R: RulesTrait> Tracer<'a, R> {
    pub fn new(layout: &mut Layout<R>) -> Tracer<R> {
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
    ) -> Result<BandIndex, DrawException> {
        Draw::new(self.layout).finish_in_dot(trace.head, into, width)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        trace: &mut Trace,
        path: &[NavvertexIndex],
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
        path: &[NavvertexIndex],
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
        to: NavvertexIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        trace.head = self.wrap(trace.head, to, width)?.into();
        trace.path.push(to);

        Ok::<(), DrawException>(())
    }

    fn wrap(
        &mut self,
        head: Head,
        around: NavvertexIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        match around {
            NavvertexIndex::FixedDot(dot) => self.wrap_around_fixed_dot(head, dot, width),
            NavvertexIndex::FixedBend(_fixed_bend) => todo!(),
            NavvertexIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(head, loose_bend, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let head = Draw::new(self.layout).cane_around_dot(head, around.into(), width)?;
        Ok(head)
    }

    fn wrap_around_loose_bend(
        &mut self,
        head: Head,
        around: LooseBendIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let head = Draw::new(self.layout).cane_around_bend(head, around.into(), width)?;

        Ok(head)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, trace: &mut Trace) {
        if let Head::Cane(head) = trace.head {
            trace.head = Draw::new(self.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }
}
