use contracts::debug_ensures;

use crate::{
    draw::{BareHead, Draw, Head},
    layout::Layout,
    mesh::{Mesh, MeshEdgeReference, VertexIndex},
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

    pub fn start(&mut self, from: VertexIndex) -> Trace {
        Trace {
            path: vec![from],
            head: Head::from(BareHead {
                dot: self.mesh.dot(from),
            }),
        }
    }

    pub fn finish(&mut self, trace: &mut Trace, into: VertexIndex, width: f64) -> Result<(), ()> {
        let into_dot = self.mesh.dot(into);
        self.draw().finish(trace.head, into_dot, width)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        trace: &mut Trace,
        path: &[VertexIndex],
        width: f64,
    ) -> Result<(), ()> {
        let prefix_length = trace
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = trace.path.len();
        self.undo_path(trace, length - prefix_length)?;
        self.path(trace, &path[prefix_length..], width)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + path.len()))]
    pub fn path(&mut self, trace: &mut Trace, path: &[VertexIndex], width: f64) -> Result<(), ()> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = self.step(trace, *vertex, width) {
                self.undo_path(trace, i)?;
                return Err(err);
            }
        }
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() - step_count))]
    pub fn undo_path(&mut self, trace: &mut Trace, step_count: usize) -> Result<(), ()> {
        for _ in 0..step_count {
            self.undo_step(trace)?;
        }
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn step(&mut self, trace: &mut Trace, to: VertexIndex, width: f64) -> Result<(), ()> {
        let to_dot = self.mesh.dot(to);
        trace.head = Head::from(self.draw().segbend_around_dot(trace.head, to_dot, width)?);
        trace.path.push(to);
        Ok(())
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() - 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn undo_step(&mut self, trace: &mut Trace) -> Result<(), ()> {
        if let Head::Segbend(head) = trace.head {
            trace.head = self.draw().undo_segbend(head).unwrap();
        } else {
            return Err(());
        }

        trace.path.pop();
        Ok(())
    }

    fn draw(&mut self) -> Draw {
        Draw::new(&mut self.layout, &self.rules)
    }
}
