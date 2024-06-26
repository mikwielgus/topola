use contracts::{debug_ensures, debug_requires};
use petgraph::{
    data::DataMap,
    graph::{NodeIndex, UnGraph},
};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandFirstSegIndex,
        bend::LooseBendIndex,
        dot::FixedDotIndex,
        graph::PrimitiveIndex,
        guide::{BareHead, CaneHead, Head},
        rules::RulesTrait,
    },
    layout::Layout,
    router::{
        draw::{Draw, DrawException},
        navmesh::{BinavvertexNodeIndex, Navmesh, NavvertexIndex, NavvertexWeight},
    },
};

#[derive(Error, Debug, Clone, Copy)]
pub enum TracerException {
    #[error(transparent)]
    CannotDraw(#[from] DrawException),
    #[error("cannot wrap")]
    CannotWrap,
}

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

    pub fn start(
        &mut self,
        _navmesh: &Navmesh,
        source: FixedDotIndex,
        source_navvertex: NavvertexIndex,
        width: f64,
    ) -> Trace {
        Trace {
            path: vec![source_navvertex],
            head: BareHead { dot: source }.into(),
            width,
        }
    }

    pub fn finish(
        &mut self,
        _navmesh: &Navmesh,
        trace: &mut Trace,
        target: FixedDotIndex,
        width: f64,
    ) -> Result<BandFirstSegIndex, TracerException> {
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
        self.undo_path(navmesh, trace, length - prefix_length);
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
            if let Err(err) = self.step(navmesh, trace, *vertex, width) {
                self.undo_path(navmesh, trace, i);
                return Err(err.into());
            }
        }

        Ok(())
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - step_count))]
    pub fn undo_path(&mut self, navmesh: &Navmesh, trace: &mut Trace, step_count: usize) {
        for _ in 0..step_count {
            self.undo_step(navmesh, trace);
        }
    }

    #[debug_ensures(ret.is_ok() -> matches!(trace.head, Head::Cane(..)))]
    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn step(
        &mut self,
        navmesh: &Navmesh,
        trace: &mut Trace,
        to: NavvertexIndex,
        width: f64,
    ) -> Result<(), TracerException> {
        trace.head = self.wrap(navmesh, trace.head, to, width)?.into();
        trace.path.push(to);

        Ok::<(), TracerException>(())
    }

    fn wrap(
        &mut self,
        navmesh: &Navmesh,
        head: Head,
        around: NavvertexIndex,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        let cw = self
            .maybe_cw(navmesh, around)
            .ok_or(TracerException::CannotWrap)?;

        match self.binavvertex(navmesh, around) {
            BinavvertexNodeIndex::FixedDot(dot) => {
                self.wrap_around_fixed_dot(navmesh, head, dot, cw, width)
            }
            BinavvertexNodeIndex::FixedBend(_fixed_bend) => todo!(),
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(navmesh, head, loose_bend, cw, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        _navmesh: &Navmesh,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(self.layout).cane_around_dot(head, around.into(), cw, width)?)
    }

    fn wrap_around_loose_bend(
        &mut self,
        _navmesh: &Navmesh,
        head: Head,
        around: LooseBendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(self.layout).cane_around_bend(head, around.into(), cw, width)?)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, _navmesh: &Navmesh, trace: &mut Trace) {
        if let Head::Cane(head) = trace.head {
            trace.head = Draw::new(self.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }

    fn maybe_cw(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> Option<bool> {
        navmesh.node_weight(navvertex).unwrap().maybe_cw
    }

    fn binavvertex(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> BinavvertexNodeIndex {
        navmesh.node_weight(navvertex).unwrap().node
    }

    fn primitive(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> PrimitiveIndex {
        self.binavvertex(navmesh, navvertex).into()
    }
}
