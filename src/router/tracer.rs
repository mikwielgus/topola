use contracts::{debug_ensures, debug_requires};
use petgraph::graph::{NodeIndex, UnGraph};
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
        navmesh::{BinavvertexNodeIndex, NavvertexWeight},
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
    pub path: Vec<NodeIndex<usize>>,
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
        _graph: &UnGraph<NavvertexWeight, (), usize>,
        source: FixedDotIndex,
        source_navvertex: NodeIndex<usize>,
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
        graph: &UnGraph<NavvertexWeight, (), usize>,
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
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        path: &[NodeIndex<usize>],
        width: f64,
    ) -> Result<(), TracerException> {
        let prefix_length = trace
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = trace.path.len();
        self.undo_path(graph, trace, length - prefix_length);
        Ok::<(), TracerException>(self.path(graph, trace, &path[prefix_length..], width)?)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + path.len()))]
    pub fn path(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        path: &[NodeIndex<usize>],
        width: f64,
    ) -> Result<(), TracerException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = self.step(graph, trace, *vertex, width) {
                self.undo_path(graph, trace, i);
                return Err(err.into());
            }
        }

        Ok(())
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - step_count))]
    pub fn undo_path(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        step_count: usize,
    ) {
        for _ in 0..step_count {
            self.undo_step(graph, trace);
        }
    }

    #[debug_ensures(ret.is_ok() -> matches!(trace.head, Head::Cane(..)))]
    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn step(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        to: NodeIndex<usize>,
        width: f64,
    ) -> Result<(), TracerException> {
        trace.head = self.wrap(graph, trace.head, to, width)?.into();
        trace.path.push(to);

        Ok::<(), TracerException>(())
    }

    fn wrap(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: NodeIndex<usize>,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        let cw = self
            .maybe_cw(graph, around)
            .ok_or(TracerException::CannotWrap)?;

        match self.binavvertex(graph, around) {
            BinavvertexNodeIndex::FixedDot(dot) => {
                self.wrap_around_fixed_dot(graph, head, dot, cw, width)
            }
            BinavvertexNodeIndex::FixedBend(_fixed_bend) => todo!(),
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(graph, head, loose_bend, cw, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(self.layout).cane_around_dot(head, around.into(), cw, width)?)
    }

    fn wrap_around_loose_bend(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: LooseBendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(self.layout).cane_around_bend(head, around.into(), cw, width)?)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, graph: &UnGraph<NavvertexWeight, (), usize>, trace: &mut Trace) {
        if let Head::Cane(head) = trace.head {
            trace.head = Draw::new(self.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }

    fn maybe_cw(
        &self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        navvertex: NodeIndex<usize>,
    ) -> Option<bool> {
        graph.node_weight(navvertex).unwrap().maybe_cw
    }

    fn binavvertex(
        &self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        navvertex: NodeIndex<usize>,
    ) -> BinavvertexNodeIndex {
        graph.node_weight(navvertex).unwrap().node
    }

    fn primitive(
        &self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        navvertex: NodeIndex<usize>,
    ) -> PrimitiveIndex {
        self.binavvertex(graph, navvertex).into()
    }
}
