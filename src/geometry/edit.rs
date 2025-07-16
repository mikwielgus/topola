// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{btree_map::Entry, BTreeMap};

use crate::graph::{GenericIndex, GetPetgraphIndex};

use super::{AccessBendWeight, AccessDotWeight, AccessSegWeight, GetLayer};

pub trait ApplyGeometryEdit<
    DW: AccessDotWeight + GetLayer,
    SW: AccessSegWeight + GetLayer,
    BW: AccessBendWeight + GetLayer,
    CW: Clone,
    Cel: Copy,
    PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
    DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
>
{
    fn apply(&mut self, edit: &GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>);
}

#[derive(Debug, Clone)]
pub struct GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI> {
    pub(super) dots: BTreeMap<DI, (Option<DW>, Option<DW>)>,
    pub(super) segs: BTreeMap<SI, (Option<((DI, DI), SW)>, Option<((DI, DI), SW)>)>,
    pub(super) bends: BTreeMap<
        BI,
        (
            Option<((DI, DI, DI, Option<BI>), BW)>,
            Option<((DI, DI, DI, Option<BI>), BW)>,
        ),
    >,
    pub(super) compounds:
        BTreeMap<GenericIndex<CW>, (Option<(Vec<(Cel, PI)>, CW)>, Option<(Vec<(Cel, PI)>, CW)>)>,
}

impl<
        DW: AccessDotWeight + GetLayer,
        SW: AccessSegWeight + GetLayer,
        BW: AccessBendWeight + GetLayer,
        CW: Clone,
        Cel: Copy,
        PI: GetPetgraphIndex + TryInto<DI> + TryInto<SI> + TryInto<BI> + Eq + Ord + Copy,
        DI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        SI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
        BI: GetPetgraphIndex + Into<PI> + Eq + Ord + Copy,
    > GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            dots: BTreeMap::new(),
            segs: BTreeMap::new(),
            bends: BTreeMap::new(),
            compounds: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, edit: GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>) {
        Self::merge_btmap(&mut self.dots, &edit.dots);
        Self::merge_btmap(&mut self.segs, &edit.segs);
        Self::merge_btmap(&mut self.bends, &edit.bends);
        Self::merge_btmap(&mut self.compounds, &edit.compounds);
    }

    fn merge_btmap<I: Copy + Eq + Ord, D: Clone>(
        main: &mut BTreeMap<I, (Option<D>, Option<D>)>,
        edit: &BTreeMap<I, (Option<D>, Option<D>)>,
    ) {
        for (index, (old, new)) in edit {
            // TODO: Delete `(None, None)`s.
            match main.entry(*index) {
                Entry::Vacant(vac) => {
                    vac.insert((old.clone(), new.clone()));
                }
                Entry::Occupied(mut occ) => {
                    occ.get_mut().1 = new.clone();
                }
            }
        }
    }

    pub fn reverse_inplace(&mut self) {
        self.dots.values_mut().for_each(Self::swap_tuple_inplace);
        self.segs.values_mut().for_each(Self::swap_tuple_inplace);
        self.bends.values_mut().for_each(Self::swap_tuple_inplace);
        self.compounds
            .values_mut()
            .for_each(Self::swap_tuple_inplace);
    }

    fn swap_tuple_inplace<D>(x: &mut (D, D)) {
        core::mem::swap(&mut x.0, &mut x.1);
    }

    pub fn reverse(&self) -> Self {
        let mut rev = self.clone();
        rev.reverse_inplace();
        rev
    }
}
