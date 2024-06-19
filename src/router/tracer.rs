use contracts::{debug_ensures, debug_requires};
use petgraph::graph::{NodeIndex, UnGraph};

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
        source_vertex: NodeIndex<usize>,
        width: f64,
    ) -> Trace {
        Trace {
            path: vec![source_vertex],
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
    ) -> Result<BandFirstSegIndex, DrawException> {
        Draw::new(self.layout).finish_in_dot(trace.head, target, width)
    }

    #[debug_requires(path[0] == trace.path[0])]
    #[debug_ensures(ret.is_ok() -> trace.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        path: &[NodeIndex<usize>],
        width: f64,
    ) -> Result<(), DrawException> {
        let prefix_length = trace
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = trace.path.len();
        self.undo_path(graph, trace, length - prefix_length);
        self.path(graph, trace, &path[prefix_length..], width)
    }

    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + path.len()))]
    pub fn path(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        path: &[NodeIndex<usize>],
        width: f64,
    ) -> Result<(), DrawException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = self.step(graph, trace, *vertex, width) {
                self.undo_path(graph, trace, i);
                return Err(err);
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

    #[debug_ensures(ret.is_ok() -> matches!(trace.head, Head::Bare(..)))]
    #[debug_ensures(ret.is_ok() -> trace.path.len() == old(trace.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> trace.path.len() == old(trace.path.len()))]
    pub fn step(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        trace: &mut Trace,
        to: NodeIndex<usize>,
        width: f64,
    ) -> Result<(), DrawException> {
        trace.head = self.wrap(graph, trace.head, to, width)?.into();
        trace.path.push(to);

        Ok::<(), DrawException>(())
    }

    fn wrap(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: NodeIndex<usize>,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        match self.binavvertex(graph, around) {
            BinavvertexNodeIndex::FixedDot(dot) => {
                self.wrap_around_fixed_dot(graph, head, dot, width)
            }
            BinavvertexNodeIndex::FixedBend(_fixed_bend) => todo!(),
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(graph, head, loose_bend, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: FixedDotIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let head = Draw::new(self.layout).cane_around_dot(head, around.into(), width)?;
        Ok(head)
    }

    fn wrap_around_loose_bend(
        &mut self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        head: Head,
        around: LooseBendIndex,
        width: f64,
    ) -> Result<CaneHead, DrawException> {
        let head = Draw::new(self.layout).cane_around_bend(head, around.into(), width)?;

        Ok(head)
    }

    #[debug_ensures(trace.path.len() == old(trace.path.len() - 1))]
    pub fn undo_step(&mut self, graph: &UnGraph<NavvertexWeight, (), usize>, trace: &mut Trace) {
        if let Head::Cane(head) = dbg!(trace.head) {
            trace.head = Draw::new(self.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        trace.path.pop();
    }

    fn binavvertex(
        &self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        vertex: NodeIndex<usize>,
    ) -> BinavvertexNodeIndex {
        graph.node_weight(vertex).unwrap().node
    }

    fn primitive(
        &self,
        graph: &UnGraph<NavvertexWeight, (), usize>,
        vertex: NodeIndex<usize>,
    ) -> PrimitiveIndex {
        self.binavvertex(graph, vertex).into()
    }
}
