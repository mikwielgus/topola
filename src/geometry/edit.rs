// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::collections::{btree_map::Entry, BTreeMap};

use crate::graph::{GenericIndex, GetPetgraphIndex};

use super::{AccessBendWeight, AccessDotWeight, AccessSegWeight, GetLayer};

pub trait Edit: Sized {
    fn reverse(&self) -> Self
    where
        Self: Clone,
    {
        let mut rev = self.clone();
        rev.reverse_inplace();
        rev
    }

    fn reverse_inplace(&mut self);

    fn merge(&mut self, edit: Self);
}

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
    > Edit for GeometryEdit<DW, SW, BW, CW, Cel, PI, DI, SI, BI>
{
    fn reverse_inplace(&mut self) {
        self.dots.reverse_inplace();
        self.segs.reverse_inplace();
        self.bends.reverse_inplace();
        self.compounds.reverse_inplace();
    }

    fn merge(&mut self, edit: Self) {
        self.dots.merge(edit.dots);
        self.segs.merge(edit.segs);
        self.bends.merge(edit.bends);
        self.compounds.merge(edit.compounds);
    }
}

impl<K: Eq + Ord, V> Edit for BTreeMap<K, (Option<V>, Option<V>)> {
    fn reverse_inplace(&mut self) {
        self.values_mut()
            .for_each(|x| core::mem::swap(&mut x.0, &mut x.1));
    }

    fn merge(&mut self, edit: Self) {
        for (index, (old, new)) in edit {
            match self.entry(index) {
                Entry::Vacant(vac) => {
                    vac.insert((old, new));
                }
                Entry::Occupied(mut occ) => match (occ.get(), new) {
                    ((None, _), None) => {
                        occ.remove();
                    }
                    (_, new) => {
                        occ.get_mut().1 = new;
                    }
                },
            }
        }
    }
}
