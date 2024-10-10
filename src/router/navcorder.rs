use contracts_try::{debug_ensures, debug_requires};
use thiserror::Error;

use crate::{
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, rules::AccessRules},
    layout::Layout,
};

use super::{
    draw::{Draw, DrawException},
    navcord::{NavcordStepContext, NavcordStepper},
    navmesh::{Navmesh, NavvertexIndex},
};

#[derive(Error, Debug, Clone, Copy)]
pub enum NavcorderException {
    #[error(transparent)]
    CannotDraw(#[from] DrawException),
    #[error("cannot wrap")]
    CannotWrap,
}

#[derive(Debug)]
pub struct Navcorder<'a, R: AccessRules> {
    pub layout: &'a mut Layout<R>,
}

impl<'a, R: AccessRules> Navcorder<'a, R> {
    pub fn new(layout: &mut Layout<R>) -> Navcorder<R> {
        Navcorder { layout }
    }

    pub fn start(
        &mut self,
        source: FixedDotIndex,
        source_navvertex: NavvertexIndex,
        width: f64,
    ) -> NavcordStepper {
        NavcordStepper::new(source, source_navvertex, width)
    }

    pub fn finish(
        &mut self,
        _navmesh: &Navmesh,
        navcord: &mut NavcordStepper,
        target: FixedDotIndex,
        width: f64,
    ) -> Result<BandTermsegIndex, NavcorderException> {
        Ok(Draw::new(self.layout).finish_in_dot(navcord.head, target, width)?)
    }

    #[debug_requires(path[0] == navcord.path[0])]
    #[debug_ensures(ret.is_ok() -> navcord.path.len() == path.len())]
    pub fn rework_path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut NavcordStepper,
        path: &[NavvertexIndex],
        width: f64,
    ) -> Result<(), NavcorderException> {
        let prefix_length = navcord
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = navcord.path.len();
        self.undo_path(navcord, length - prefix_length);
        self.path(navmesh, navcord, &path[prefix_length..], width)
    }

    #[debug_ensures(ret.is_ok() -> navcord.path.len() == old(navcord.path.len() + path.len()))]
    pub fn path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut NavcordStepper,
        path: &[NavvertexIndex],
        width: f64,
    ) -> Result<(), NavcorderException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = navcord.step(&mut NavcordStepContext {
                navcorder: self,
                navmesh,
                to: *vertex,
                width,
            }) {
                self.undo_path(navcord, i);
                return Err(err);
            }
        }

        Ok(())
    }

    #[debug_ensures(navcord.path.len() == old(navcord.path.len() - step_count))]
    pub fn undo_path(&mut self, navcord: &mut NavcordStepper, step_count: usize) {
        for _ in 0..step_count {
            let _ = navcord.step_back(self);
        }
    }
}
