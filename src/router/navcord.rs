use contracts_try::debug_ensures;
use petgraph::data::DataMap;

use crate::drawing::{
    bend::LooseBendIndex,
    dot::FixedDotIndex,
    graph::PrimitiveIndex,
    head::{BareHead, CaneHead, Head},
    rules::AccessRules,
};

use super::{
    draw::Draw,
    navcorder::{Navcorder, NavcorderException},
    navmesh::{BinavvertexNodeIndex, Navmesh, NavvertexIndex},
};

#[derive(Debug)]
pub struct NavcordStepper {
    pub path: Vec<NavvertexIndex>,
    pub head: Head,
    pub width: f64,
}

impl NavcordStepper {
    pub fn new(
        source: FixedDotIndex,
        source_navvertex: NavvertexIndex,
        width: f64,
    ) -> NavcordStepper {
        Self {
            path: vec![source_navvertex],
            head: BareHead { face: source }.into(),
            width,
        }
    }

    fn wrap(
        &mut self,
        navcorder: &mut Navcorder<impl AccessRules>,
        navmesh: &Navmesh,
        head: Head,
        around: NavvertexIndex,
        width: f64,
    ) -> Result<CaneHead, NavcorderException> {
        let cw = self
            .maybe_cw(navmesh, around)
            .ok_or(NavcorderException::CannotWrap)?;

        match self.binavvertex(navmesh, around) {
            BinavvertexNodeIndex::FixedDot(dot) => {
                self.wrap_around_fixed_dot(navcorder, head, dot, cw, width)
            }
            BinavvertexNodeIndex::FixedBend(_fixed_bend) => todo!(),
            BinavvertexNodeIndex::LooseBend(loose_bend) => {
                self.wrap_around_loose_bend(navcorder, head, loose_bend, cw, width)
            }
        }
    }

    fn wrap_around_fixed_dot(
        &mut self,
        navcorder: &mut Navcorder<impl AccessRules>,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, NavcorderException> {
        Ok(Draw::new(navcorder.layout).cane_around_dot(head, around, cw, width)?)
    }

    fn wrap_around_loose_bend(
        &mut self,
        navcorder: &mut Navcorder<impl AccessRules>,
        head: Head,
        around: LooseBendIndex,
        cw: bool,
        width: f64,
    ) -> Result<CaneHead, NavcorderException> {
        Ok(Draw::new(navcorder.layout).cane_around_bend(head, around.into(), cw, width)?)
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

pub struct NavcordStepContext<'a: 'b, 'b, R: AccessRules> {
    pub navcorder: &'b mut Navcorder<'a, R>,
    pub navmesh: &'b Navmesh,
    pub to: NavvertexIndex,
    pub width: f64,
}

impl NavcordStepper {
    #[debug_ensures(ret.is_ok() -> matches!(self.head, Head::Cane(..)))]
    #[debug_ensures(ret.is_ok() -> self.path.len() == old(self.path.len() + 1))]
    #[debug_ensures(ret.is_err() -> self.path.len() == old(self.path.len()))]
    pub fn step<'a, 'b, R: AccessRules>(
        &mut self,
        input: &mut NavcordStepContext<'a, 'b, R>,
    ) -> Result<(), NavcorderException> {
        self.head = self
            .wrap(
                input.navcorder,
                input.navmesh,
                self.head,
                input.to,
                input.width,
            )?
            .into();
        self.path.push(input.to);

        Ok(())
    }

    #[debug_ensures(self.path.len() == old(self.path.len() - 1))]
    pub fn step_back<'a, R: AccessRules>(
        &mut self,
        navcorder: &mut Navcorder<'a, R>,
    ) -> Result<(), NavcorderException> {
        if let Head::Cane(head) = self.head {
            self.head = Draw::new(navcorder.layout).undo_cane(head).unwrap();
        } else {
            panic!();
        }

        self.path.pop();
        Ok(())
    }
}
