use contracts::debug_ensures;

use crate::{
    draw::{BareHead, Draw, Head, SegbendHead},
    graph::FixedDotIndex,
    layout::Layout,
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

    pub fn start(&mut self, from: FixedDotIndex) -> Trace {
        Trace {
            path: vec![from.into()],
            head: BareHead { dot: from }.into(),
        }
    }

    pub fn finish(&mut self, trace: &mut Trace, into: FixedDotIndex, width: f64) -> Result<(), ()> {
        self.draw().finish_in_dot(trace.head, into, width)
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
        trace.head = self.wrap(trace.head, to, width)?.into();

        trace.path.push(to);
        Ok(())
    }

    fn wrap(&mut self, head: Head, around: VertexIndex, width: f64) -> Result<SegbendHead, ()> {
        /*let _around_pos = self.layout.primitive(around).weight().circle.pos;
        let _around_primitive = self.layout.primitive(around);

        'blk: {
            if let Some(mut layer) = self.layout.primitive(around).outer() {
                match self.is_under(head, around, layer) {
                    Some(true) => return self.tuck_around_dot(head, around, width),
                    Some(false) => (),
                    None => break 'blk,
                }

                while let Some(outer) = self.layout.primitive(layer).outer() {
                    match self.is_under(head, around, outer) {
                        Some(true) => return self.tuck_around_bend(head, layer, width),
                        Some(false) => (),
                        None => break 'blk,
                    }

                    layer = outer;
                }

                return self.draw().segbend_around_bend(head, layer.into(), width);
            }
        }*/

        match around {
            VertexIndex::FixedDot(dot) => self.wrap_around_fixed_dot(head, dot, width),
            VertexIndex::FixedBend(_fixed_bend) => todo!(),
            VertexIndex::LooseBend(_loose_bend) => todo!(),
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        self.draw().segbend_around_dot(head, around.into(), width)
    }

    /*fn is_under(
        &mut self,
        _head: Head,
        around: FixedDotIndex,
        _layer: FixedBendIndex,
    ) -> Option<bool> {
        let _around_pos = self.layout.primitive(around).weight().circle.pos;

        /*if Some(layer) != self.layout.primitive(head.dot()).prev_bend() {
            Some(
                self.layout
                    .primitive(layer)
                    .shape()
                    .into_bend()
                    .unwrap()
                    .between_ends(around_pos),
            )
        } else {*/
        None
        //}
    }*/

    /*fn tuck_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self
            .draw()
            .segbend_around_dot(Head::from(head), around.into(), width)?;
        self.layout.reattach_bend(outer, head.segbend.bend);

        self.redraw_outward(outer)?;
        Ok(head)
    }

    fn tuck_around_bend(
        &mut self,
        head: Head,
        around: FixedBendIndex,
        width: f64,
    ) -> Result<SegbendHead, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self
            .draw()
            .segbend_around_bend(Head::from(head), around.into(), width)?;
        self.layout.reattach_bend(outer, head.segbend.bend);

        self.redraw_outward(outer)?;
        Ok(head)
    }

    fn redraw_outward(&mut self, bend: FixedBendIndex) -> Result<(), ()> {
        let mut bows: Vec<Bow> = vec![];

        let mut cur_bend = bend;
        loop {
            bows.push(self.layout.bow(cur_bend));

            cur_bend = match self.layout.primitive(cur_bend).outer() {
                Some(new_bend) => new_bend,
                None => break,
            }
        }

        let core = self.layout.primitive(bend).core().unwrap();
        let mut maybe_inner = self.layout.primitive(bend).inner();

        for bow in &bows {
            self.layout.remove_interior(bow);
        }

        for bow in &bows {
            let ends = bow.ends();
            let head = self.draw().start(ends.0);
            let width = 5.0;

            let segbend_head = if let Some(inner) = maybe_inner {
                self.draw().segbend_around_bend(head, inner.into(), width)?
            } else {
                self.draw().segbend_around_dot(head, core.into(), width)?
            };

            maybe_inner = Some(segbend_head.segbend.bend);
            self.draw().finish(head, ends.1, width)?;
            //self.relax_band(maybe_inner.unwrap());
        }

        Ok(())
    }*/

    /*fn relax_band(&mut self, bend: FixedBendIndex) {
        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).prev_bend() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }

        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).next_bend() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }
    }*/

    /*fn release_bow(&mut self, bend: FixedBendIndex) {
        let bow = self.layout.bow(bend);
        let ends = bow.ends();

        self.layout.remove_interior(&bow);

        let head = self.draw().start(ends.0);

        let _ = self.draw().finish(head, ends.1, 5.0);
    }*/

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
