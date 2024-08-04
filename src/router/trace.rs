use contracts::debug_ensures;
use petgraph::data::DataMap;

use crate::{
    drawing::{
        bend::LooseBendIndex,
        dot::FixedDotIndex,
        graph::PrimitiveIndex,
        head::{BareHead, CaneHead, Head},
        rules::AccessRules,
    },
    step::{Step, StepBack},
};

use super::{
    draw::Draw,
    navmesh::{BinavvertexNodeIndex, Navmesh, NavvertexIndex},
    tracer::TracerStatus,
    tracer::{Tracer, TracerException},
};

#[derive(Debug)]
pub struct Trace {
    pub path: Vec<NavvertexIndex>,
    pub head: Head,
    pub width: f64,
}

impl Trace {
    pub fn new(source: FixedDotIndex, source_navvertex: NavvertexIndex, width: f64) -> Trace {
        Self {
            path: vec![source_navvertex],
            head: BareHead { face: source }.into(),
            width,
        }
    }

    fn wrap(
        &mut self,
        tracer: &mut Tracer<impl AccessRules>,
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
                self.wrap_around_fixed_dot(tracer, head, dot, cw, width)
            }
            BinavvertexNodeIndex::FixedBend(_fixed_bend) => todo!(),
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(tracer, head, loose_bend, cw, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        tracer: &mut Tracer<impl AccessRules>,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(tracer.layout).cane_around_dot(head, around.into(), cw, width)?)
    }

    fn wrap_around_loose_bend(
        &mut self,
        tracer: &mut Tracer<impl AccessRules>,
        head: Head,
        around: LooseBendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, TracerException> {
        Ok(Draw::new(tracer.layout).cane_around_bend(head, around.into(), cw, width)?)
    }

    fn binavvertex(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> BinavvertexNodeIndex {
        navmesh.node_weight(navvertex).unwrap().node
    }

    fn primitive(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> PrimitiveIndex {
        self.binavvertex(navmesh, navvertex).into()
    }

    fn maybe_cw(&self, navmesh: &Navmesh, navvertex: NavvertexIndex) -> Option<bool> {
        navmesh.node_weight(navvertex).unwrap().maybe_cw
    }
}

pub struct TraceStepContext<'a: 'b, 'b, R: AccessRules> {
    pub tracer: &'b mut Tracer<'a, R>,
    pub navmesh: &'b Navmesh,
    pub to: NavvertexIndex,
    pub width: f64,
}

impl<'a, 'b, R: AccessRules> Step<TraceStepContext<'a, 'b, R>, TracerStatus, TracerException, ()>
    for Trace
{
    #[debug_ensures(ret.is_ok() -> matches!(self.head, Head::Cane(..)))]
    #[debug_ensures(ret.is_ok() -> self.path.len() == old(self.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> self.path.len() == old(self.path.len()))]
    fn step(
        &mut self,
        input: &mut TraceStepContext<'a, 'b, R>,
    ) -> Result<TracerStatus, TracerException> {
        self.head = self
            .wrap(
                input.tracer,
                input.navmesh,
                self.head,
                input.to,
                input.width,
            )?
            .into();
        self.path.push(input.to);

        Ok(TracerStatus::Running)
    }
}

impl<'a, R: AccessRules> StepBack<Tracer<'a, R>, TracerStatus, TracerException, ()> for Trace {
    #[debug_ensures(self.path.len() == old(self.path.len() - 1))]
    fn step_back(&mut self, tracer: &mut Tracer<'a, R>) -> Result<TracerStatus, TracerException> {
        if let Head::Cane(head) = self.head {
            self.head = Draw::new(tracer.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        self.path.pop();
        Ok(TracerStatus::Running)
    }
}
