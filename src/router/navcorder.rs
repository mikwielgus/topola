// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use contracts_try::{debug_ensures, debug_requires};
use thiserror::Error;

use crate::{
    drawing::{band::BandTermsegIndex, dot::FixedDotIndex, rules::AccessRules},
    layout::{Layout, LayoutEdit},
};

use super::{
    draw::{Draw, DrawException},
    navcord::Navcord,
    navmesh::{Navmesh, NavnodeIndex},
};

#[derive(Error, Debug, Clone, Copy)]
pub enum NavcorderException {
    #[error(transparent)]
    CannotDraw(#[from] DrawException),
    #[error("cannot wrap")]
    CannotWrap,
}

pub trait Navcorder {
    fn start(
        &mut self,
        recorder: LayoutEdit,
        source: FixedDotIndex,
        source_navnode: NavnodeIndex,
        width: f64,
    ) -> Navcord;

    fn finish(
        &mut self,
        _navmesh: &Navmesh,
        navcord: &mut Navcord,
        target: FixedDotIndex,
    ) -> Result<BandTermsegIndex, NavcorderException>;

    fn rework_path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut Navcord,
        path: &[NavnodeIndex],
    ) -> Result<(), NavcorderException>;

    fn path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut Navcord,
        path: &[NavnodeIndex],
    ) -> Result<(), NavcorderException>;

    fn undo_path(&mut self, navcord: &mut Navcord, step_count: usize);
}

impl<R: AccessRules> Navcorder for Layout<R> {
    fn start(
        &mut self,
        recorder: LayoutEdit,
        source: FixedDotIndex,
        source_navnode: NavnodeIndex,
        width: f64,
    ) -> Navcord {
        Navcord::new(recorder, source, source_navnode, width)
    }

    fn finish(
        &mut self,
        _navmesh: &Navmesh,
        navcord: &mut Navcord,
        target: FixedDotIndex,
    ) -> Result<BandTermsegIndex, NavcorderException> {
        Ok(self.finish_in_dot(&mut navcord.recorder, navcord.head, target, navcord.width)?)
    }

    #[debug_requires(path[0] == navcord.path[0])]
    #[debug_ensures(ret.is_ok() -> navcord.path.len() == path.len())]
    fn rework_path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut Navcord,
        path: &[NavnodeIndex],
    ) -> Result<(), NavcorderException> {
        let prefix_length = navcord
            .path
            .iter()
            .zip(path)
            .take_while(|(v1, v2)| v1 == v2)
            .count();

        let length = navcord.path.len();
        self.undo_path(navcord, length - prefix_length);
        self.path(navmesh, navcord, &path[prefix_length..])
    }

    #[debug_ensures(ret.is_ok() -> navcord.path.len() == old(navcord.path.len() + path.len()))]
    fn path(
        &mut self,
        navmesh: &Navmesh,
        navcord: &mut Navcord,
        path: &[NavnodeIndex],
    ) -> Result<(), NavcorderException> {
        for (i, vertex) in path.iter().enumerate() {
            if let Err(err) = navcord.step_to(self, navmesh, *vertex) {
                self.undo_path(navcord, i);
                return Err(err);
            }
        }

        Ok(())
    }

    #[debug_ensures(navcord.path.len() == old(navcord.path.len() - step_count))]
    fn undo_path(&mut self, navcord: &mut Navcord, step_count: usize) {
        for _ in 0..step_count {
            let _ = navcord.step_back(self);
        }
    }
}
