use contracts::debug_ensures;

use crate::{
    draw::{Draw, DrawException},
    graph::{FixedDotIndex, GetNet, LooseBendIndex},
    guide::{BareHead, Head, SegbendHead},
    layout::{Band, Layout, LayoutException},
    mesh::{Mesh, VertexIndex},
    rules::Rules,
};

#[derive(Debug)]
pub struct Trace {
    pub path: Vec<VertexIndex>,
    head: Head,
}

pub struct Tracer<'a> {
    pub layout: &'a mut Layout,
    pub rules: &'a Rules,
    pub mesh: &'a Mesh,
}

impl<'a> Tracer<'a> {
    pub fn new(layout: &'a mut Layout, rules: &'a Rules, mesh: &'a Mesh) -> Self {
        Tracer {
            layout,
            rules,
            mesh,
        }
    }

    pub fn start(&mut self, from: FixedDotIndex, width: f64) -> Trace {
        let band = self.layout.bands.insert(Band {
            width,
            net: self.layout.primitive(from).net(),
        });
        Trace {
            path: vec![from.into()],
            head: BareHead { dot: from, band }.into(),
        }
    }

    pub fn finish(
        &mut self,
        trace: &mut Trace,
        into: FixedDotIndex,
        width: f64,
    ) -> Result<(), DrawException> {
        self.draw().finish_in_dot(trace.head, into, width)
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
        let head = self.draw().segbend_around_dot(head, around.into(), width)?;
        Ok(head)
    }

    fn wrap_around_loose_bend(
        &mut self,
        head: Head,
        around: LooseBendIndex,
        width: f64,
    ) -> Result<SegbendHead, DrawException> {
        let head = self
            .draw()
            .segbend_around_bend(head, around.into(), width)?;

        Ok(head)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, trace: &mut Trace) {
        if let Head::Segbend(head) = trace.head {
            trace.head = self.draw().undo_segbend(head).unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }

    fn draw(&mut self) -> Draw {
        Draw::new(&mut self.layout, &self.rules)
    }
}
