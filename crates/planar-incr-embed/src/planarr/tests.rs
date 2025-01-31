// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! per-node planar arrangement structures
//! * `NI`... type of node indices
//! * `EP`... type of etched path descriptor

extern crate std;
use super::*;
use alloc::{boxed::Box, sync::Arc};
use insta::assert_compact_json_snapshot;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct PlanarArrangement<NI, EP, CT>(
    /// counter-clockwise (CCW) ordered sectors, containing CCW ordered paths
    pub Box<[(NI, Arc<[RelaxedPath<EP, CT>]>)]>,
);

impl<NI: Clone, EP, CT> PlanarArrangement<NI, EP, CT> {
    pub fn from_node_indices(idxs: impl Iterator<Item = NI>) -> Self {
        let empty_edge: Arc<[RelaxedPath<EP, CT>]> = Arc::from(Vec::new().into_boxed_slice());
        Self(idxs.map(|i| (i, empty_edge.clone())).collect())
    }

    pub fn as_mut(&mut self) -> PlanarArrangementRefMut<'_, NI, EP, CT> {
        PlanarArrangementRefMut(
            self.0
                .iter_mut()
                .map(|(i, j)| (i.clone(), MaybeReversed::new(j)))
                .collect(),
        )
    }
}

struct PlanarArrangementRefMut<'a, NI, EP, CT>(
    /// counter-clockwise (CCW) ordered sectors, containing CCW ordered paths
    pub  Box<
        [(
            NI,
            MaybeReversed<&'a mut Arc<[RelaxedPath<EP, CT>]>, RelaxedPath<EP, CT>>,
        )],
    >,
);

impl<NI: Clone + Eq, EP: Clone + Eq, CT> PlanarArrangementRefMut<'_, NI, EP, CT> {
    /// See [`find_other_end`].
    #[inline(always)]
    pub fn find_other_end(
        &self,
        start: &NI,
        pos: usize,
        already_inserted_at_start: bool,
        stop: &NI,
    ) -> Option<(usize, OtherEnd)> {
        find_other_end(
            self.0.iter().map(|(i, j)| (i.clone(), j.as_ref())),
            start,
            pos,
            already_inserted_at_start,
            stop,
        )
    }

    /// See [`find_all_other_ends`].
    #[inline(always)]
    pub fn find_all_other_ends<'a>(
        &'a self,
        start: &'a NI,
        pos: usize,
        already_inserted_at_start: bool,
    ) -> Option<(usize, impl Iterator<Item = (NI, OtherEnd)> + 'a)> {
        find_all_other_ends(
            self.0.iter().map(|(i, j)| (i.clone(), j.as_ref())),
            start,
            pos,
            already_inserted_at_start,
        )
    }

    /// Insert a path into the current sectors arrangement,
    /// starting at position `pos_start` in sector `start`, and finding
    /// the appropriate other end position in `stop`, and inserting the path there.
    ///
    /// See also [`find_other_end`]
    /// (which does implement the search, look there for failure and edge cases).
    ///
    /// If given valid input `self`, this function won't make `self` invalid.
    ///
    /// The result in the success case is the inverted stop position
    /// (which can be passed to the next `insert_path` of the neighbor `stop`'s node)
    pub fn insert_path(
        &mut self,
        start: &NI,
        pos_start: usize,
        stop: &NI,
        path: EP,
    ) -> Result<usize, EP>
    where
        CT: Clone,
    {
        match self.find_other_end(start, pos_start, false, stop) {
            None => Err(path),
            Some((idx_start, stop_data)) => {
                let path = RelaxedPath::Normal(path);

                self.0[idx_start].1.with_borrow_mut(|mut j| {
                    j.insert(pos_start, path.clone());
                });

                self.0[stop_data.section_idx].1.with_borrow_mut(|mut j| {
                    j.insert(stop_data.insert_pos, path);
                    Ok(j.len() - stop_data.insert_pos - 1)
                })
            }
        }
    }
}

#[test]
fn simple00() {
    let mut s = PlanarArrangement::<_, _, ()>::from_node_indices(0..3);
    let mut s_ = s.as_mut();
    assert_eq!(s_.insert_path(&0, 0, &1, 'a'), Ok(0));
    assert_eq!(s_.insert_path(&0, 0, &1, 'b'), Ok(0));
    {
        let tmp = s_.find_all_other_ends(&0, 0, false).unwrap();
        assert_compact_json_snapshot!((tmp.0, tmp.1.collect::<Vec<_>>()));
    }
    assert_eq!(s_.insert_path(&0, 0, &2, 'c'), Ok(0));
    {
        let tmp = s_.find_all_other_ends(&1, 2, false).unwrap();
        assert_compact_json_snapshot!((tmp.0, tmp.1.collect::<Vec<_>>()));
    }
    assert_eq!(s_.insert_path(&1, 2, &2, 'd'), Ok(1));
    assert_compact_json_snapshot!(s.0);
}

#[test]
fn simple01() {
    let mut s = PlanarArrangement::<_, _, ()>::from_node_indices(0..3);
    let mut s_ = s.as_mut();
    s_.0[2].1.reversed = true;
    assert_eq!(s_.insert_path(&0, 0, &1, 'a'), Ok(0));
    assert_eq!(s_.insert_path(&0, 0, &1, 'b'), Ok(0));
    assert_eq!(s_.insert_path(&0, 0, &2, 'c'), Ok(0));
    assert_eq!(s_.insert_path(&1, 2, &2, 'd'), Ok(1));
    assert_compact_json_snapshot!(s.0);
}
