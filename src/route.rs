use crate::{
    draw::{Draw, Head},
    layout::Layout,
    mesh::{Mesh, VertexIndex},
    rules::Rules,
};

#[derive(Debug)]
pub struct Trace {
    path: Vec<VertexIndex>,
    head: Head,
}

pub struct Route<'a> {
    layout: &'a mut Layout,
    rules: &'a Rules,
    mesh: &'a Mesh,
}

impl<'a> Route<'a> {
    pub fn new(layout: &'a mut Layout, rules: &'a Rules, mesh: &'a Mesh) -> Self {
        Route {
            layout,
            rules,
            mesh,
        }
    }

    pub fn start(&mut self, from: VertexIndex) -> Trace {
        Trace {
            path: vec![from],
            head: Head {
                dot: self.mesh.dot(from),
                segbend: None,
            },
        }
    }

    pub fn finish(&mut self, trace: &mut Trace, into: VertexIndex, width: f64) -> Result<(), ()> {
        let into_dot = self.mesh.dot(into);
        self.draw().finish(trace.head, into_dot, width)
    }

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

    pub fn path(&mut self, trace: &mut Trace, path: &[VertexIndex], width: f64) -> Result<(), ()> {
        for vertex in path {
            self.step(trace, *vertex, width)?;
        }
        Ok(())
    }

    pub fn undo_path(&mut self, trace: &mut Trace, step_count: usize) -> Result<(), ()> {
        for _ in 0..step_count {
            self.undo_step(trace)?;
        }
        Ok(())
    }

    pub fn step(&mut self, trace: &mut Trace, to: VertexIndex, width: f64) -> Result<(), ()> {
        let to_dot = self.mesh.dot(to);
        trace.head = self
            .draw()
            .segbend_around_dot(trace.head, to_dot, true, width)?;
        trace.path.push(to);
        Ok(())
    }

    pub fn undo_step(&mut self, trace: &mut Trace) -> Result<(), ()> {
        trace.head = self.draw().undo_segbend(trace.head).unwrap();
        trace.path.pop();
        Ok(())
    }

    fn draw(&mut self) -> Draw {
        Draw::new(&mut self.layout, &self.rules)
    }
}
