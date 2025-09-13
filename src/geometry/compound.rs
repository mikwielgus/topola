// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use crate::graph::{GenericIndex, GetIndex};

pub trait ManageCompounds<CW: Clone> {
    type GeneralIndex: Copy;
    type EntryLabel: Copy;

    fn add_compound(&mut self, weight: CW) -> GenericIndex<CW>;
    fn remove_compound(&mut self, compound: GenericIndex<CW>);
    fn add_to_compound<I>(&mut self, node: I, label: Self::EntryLabel, compound: GenericIndex<CW>)
    where
        I: Copy + GetIndex;

    fn compound_weight(&self, node: GenericIndex<CW>) -> &CW;

    fn compound_members(
        &self,
        compound: GenericIndex<CW>,
    ) -> impl Iterator<Item = (Self::EntryLabel, Self::GeneralIndex)> + '_;

    fn compounds<I>(&self, node: I) -> impl Iterator<Item = (Self::EntryLabel, GenericIndex<CW>)>
    where
        I: Copy + GetIndex;
}
