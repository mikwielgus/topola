// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! per-node planar arrangement structures
//! * `NI`... type of node indices
//! * `EP`... type of etched path descriptor

use crate::{
    mayrev::MaybeReversed,
    utils::{handle_lifo_relaxed, rotate_iter},
    RelaxedPath,
};
use alloc::vec::Vec;

/// Data about the other end of a path going through an planar arrangement
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "serde"),
    derive(serde::Deserialize, serde::Serialize)
)]
pub struct OtherEnd {
    /// the index / position of the target section
    pub section_idx: usize,
    /// the position to be used inside the target section
    pub insert_pos: usize,
}

/// Find the appropriate other end (in `stop`) position of a path
/// starting between `pos - 1` and `pos`
/// (at `pos` exactly if `already_inserted_at_start` is set) in `start`.
/// Returns `Option<start_idx, stop_data>`.
///
/// ## Edge cases
/// Note that this function "works" if `start == stop`, but does introduce a bias
/// (currently meaning that routes are introduced before instead of after this)
/// into further routes to resolve the resulting ambiguity
/// and should therefore be avoided entirely.
///
/// ## Failure
/// If the input data is invalid:
/// * the `start` or `stop` doesn't exist, or
/// * `NI`'s are non-unique (`self.0.iter().map(|i| &i.0)` should never contain duplicates)
/// * the `[EP]`'s aren't LIFO-ordered as expected, or
/// * the `pos` is out of bounds,
///
/// this function returns `None`.
pub fn find_other_end<'a, NI, EP, CT, Iter, EPB>(
    this: Iter,
    start: &NI,
    pos: usize,
    already_inserted_at_start: bool,
    stop: &NI,
) -> Option<(usize, OtherEnd)>
where
    Iter: Clone
        + Iterator<Item = (NI, MaybeReversed<&'a EPB, RelaxedPath<EP, CT>>)>
        + core::iter::ExactSizeIterator,
    NI: Eq,
    EP: Clone + Eq + 'a,
    EPB: core::borrow::Borrow<[RelaxedPath<EP, CT>]> + 'a,
{
    let mut stack = Vec::new();

    // iteration in counter-clockwise order
    let (start_idx, mut it) = rotate_iter(
        this.map(|(i, j)| (i, j.as_ref())).enumerate(),
        move |(_, (i, _))| i == start,
    );

    // 1. handle start
    {
        let (_, (_, start_eps)) = it.next().unwrap();
        if start == stop {
            return Some((
                start_idx,
                OtherEnd {
                    section_idx: start_idx,
                    insert_pos: pos + 1,
                },
            ));
        }
        for i in start_eps
            .as_ref()
            .iter()
            .skip(pos + usize::from(already_inserted_at_start))
        {
            handle_lifo_relaxed(&mut stack, i);
        }
    }

    // 2. handle rest
    for (nni, (ni, eps)) in it {
        if &ni == stop {
            // find insertion point (one of `eps.len()+1` positions)
            return if stack.is_empty() {
                Some((
                    start_idx,
                    OtherEnd {
                        section_idx: nni,
                        insert_pos: 0,
                    },
                ))
            } else {
                for (n, i) in eps.as_ref().iter().enumerate() {
                    handle_lifo_relaxed(&mut stack, i);
                    if stack.is_empty() {
                        return Some((
                            start_idx,
                            OtherEnd {
                                section_idx: nni,
                                insert_pos: n + 1,
                            },
                        ));
                    }
                }
                None
            };
        } else {
            for i in eps.as_ref().iter() {
                handle_lifo_relaxed(&mut stack, i);
            }
        }
    }

    None
}

/// Find the appropriate other ends of a path
/// starting between `pos - 1` and `pos`
/// (at `pos` exactly if `already_inserted_at_start` is set) in `start`.
/// Returns `Option<start_idx, Vec<(stop_ni, stop_data)>>`.
///
/// ## Edge cases
/// Note that this function won't report possible entries with `start_idx == stop_idx`.
///
/// ## Failure
/// If the input data is invalid:
/// * the `start` doesn't exist, or
/// * `NI`'s are non-unique (`self.0.iter().map(|i| &i.0)` should never contain duplicates)
/// * the `[EP]`'s aren't LIFO-ordered as expected, or
/// * the `pos` is out of bounds,
///
/// this function returns `None`.
pub fn find_all_other_ends<'a, NI, EP, CT, Iter, EPB>(
    this: Iter,
    start: &'a NI,
    pos: usize,
    already_inserted_at_start: bool,
) -> Option<(usize, impl Iterator<Item = (NI, OtherEnd)> + 'a)>
where
    Iter: Clone
        + Iterator<Item = (NI, MaybeReversed<&'a EPB, RelaxedPath<EP, CT>>)>
        + core::iter::ExactSizeIterator
        + 'a,
    NI: Clone + Eq,
    EP: Clone + Eq + 'a,
    CT: 'a,
    EPB: core::borrow::Borrow<[RelaxedPath<EP, CT>]> + 'a,
{
    let mut stack = Vec::new();

    // iteration in counter-clockwise order
    let (start_idx, mut it) = rotate_iter(this.enumerate(), move |(_, (i, _))| i == start);

    // 1. handle start
    {
        let (_, (st_, start_eps)) = it.next().unwrap();
        if &st_ != start {
            panic!();
        }
        for i in start_eps
            .iter()
            .skip(pos + usize::from(already_inserted_at_start))
        {
            handle_lifo_relaxed(&mut stack, i);
        }
    }

    // 2. handle rest
    Some((
        start_idx,
        it.filter_map(move |(section_idx, (ni, eps))| {
            // find possible insertion point
            // (at most one of `eps.len()+1` positions)
            let mut pos = if stack.is_empty() { Some(0) } else { None };
            for (n, i) in eps.iter().enumerate() {
                handle_lifo_relaxed(&mut stack, i);
                if pos.is_none() && stack.is_empty() {
                    pos = Some(n + 1);
                }
            }
            pos.map(|insert_pos| {
                (
                    ni,
                    OtherEnd {
                        section_idx,
                        insert_pos,
                    },
                )
            })
        }),
    ))
}

#[cfg(test)]
mod tests;
