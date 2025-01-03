// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, marker::PhantomData};

use crate::{
    drawing::graph::{GetLayer, Retag},
    graph::{GenericIndex, GetPetgraphIndex},
};

use super::{AccessBendWeight, AccessDotWeight, AccessSegWeight, GetWidth};

pub trait ApplyGeometryEdit<
    PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
    DW: AccessDotWeight + Into<PW> + GetLayer,
    SW: AccessSegWeight + Into<PW> + GetLayer,
    BW: AccessBendWeight + Into<PW> + GetLayer,
    CW: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
    DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
>
{
    fn apply(&mut self, edit: &GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>);
}

#[derive(Debug, Clone)]
pub struct GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI> {
    pub(super) dots: BTreeMap<DI, (Option<DW>, Option<DW>)>,
    pub(super) segs: BTreeMap<SI, (Option<((DI, DI), SW)>, Option<((DI, DI), SW)>)>,
    pub(super) bends: BTreeMap<BI, (Option<((DI, DI, DI), BW)>, Option<((DI, DI, DI), BW)>)>,
    pub(super) compounds:
        BTreeMap<GenericIndex<CW>, (Option<(Vec<PI>, CW)>, Option<(Vec<PI>, CW)>)>,
    primitive_weight_marker: PhantomData<PW>,
}

impl<
        PW: GetWidth + GetLayer + TryInto<DW> + TryInto<SW> + TryInto<BW> + Retag<PI> + Copy,
        DW: AccessDotWeight + Into<PW> + GetLayer,
        SW: AccessSegWeight + Into<PW> + GetLayer,
        BW: AccessBendWeight + Into<PW> + GetLayer,
        CW: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    > GeometryEdit<PW, DW, SW, BW, CW, PI, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            dots: BTreeMap::new(),
            segs: BTreeMap::new(),
            bends: BTreeMap::new(),
            compounds: BTreeMap::new(),
            primitive_weight_marker: PhantomData,
        }
    }

    pub fn reverse(&self) -> Self {
        Self {
            dots: self
                .dots
                .clone()
                .into_iter()
                .map(|(k, v)| (k, (v.1, v.0)))
                .collect(),
            segs: self
                .segs
                .clone()
                .into_iter()
                .map(|(k, v)| (k, (v.1, v.0)))
                .collect(),
            bends: self
                .bends
                .clone()
                .into_iter()
                .map(|(k, v)| (k, (v.1, v.0)))
                .collect(),
            compounds: self
                .compounds
                .clone()
                .into_iter()
                .map(|(k, v)| (k, (v.1, v.0)))
                .collect(),
            primitive_weight_marker: PhantomData,
        }
    }
}
