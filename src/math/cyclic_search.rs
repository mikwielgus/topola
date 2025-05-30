// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

use core::{mem::take, ops::Range};

/// generate a breadth-first search list for given bounds and level
fn breadth4level(bounds: Range<usize>, level: u8, mut callback: impl FnMut(Range<usize>) -> bool) {
    // level is the exponent of 2
    let block_length = (bounds.end - bounds.start) >> level;
    let blocks_count = (bounds.end - bounds.start) / block_length;
    if blocks_count == 0 {
        return;
    }
    for i in 0..(blocks_count - 1) {
        let block_start = bounds.start + i * block_length;
        debug_assert!(block_start + block_length < bounds.end);
        if !callback(block_start..(block_start + block_length)) {
            return;
        }
    }
    let block_start = bounds.start + (blocks_count - 1) * block_length;
    debug_assert!(block_start + block_length <= bounds.end);
    callback(block_start..bounds.end);
}

#[derive(Clone, Copy)]
enum TriState<T> {
    Nothing,
    Got(T),
    Fixed(T),
}

impl<T> Default for TriState<T> {
    fn default() -> Self {
        TriState::Nothing
    }
}

impl<T> TriState<T> {
    fn update(&mut self, value: T) -> bool {
        match self {
            TriState::Fixed(_) => false,
            _ => {
                *self = TriState::Got(value);
                true
            }
        }
    }

    fn fix(&mut self) {
        *self = match take(self) {
            TriState::Got(x) => TriState::Fixed(x),
            x => x,
        };
    }

    fn is_fixed(&self) -> bool {
        matches!(self, TriState::Fixed(_))
    }
}

impl<T> From<TriState<T>> for Option<T> {
    fn from(x: TriState<T>) -> Self {
        match x {
            TriState::Nothing => None,
            TriState::Got(x) | TriState::Fixed(x) => Some(x),
        }
    }
}

struct Discover {
    pos_false: TriState<usize>,
    pos_true: TriState<usize>,
}

impl Discover {
    fn new() -> Self {
        Self {
            pos_false: TriState::Nothing,
            pos_true: TriState::Nothing,
        }
    }

    fn update(&mut self, pos: usize, value: bool) {
        match value {
            false => {
                self.pos_false.update(pos);
                self.pos_true.fix();
            }
            true => {
                self.pos_true.update(pos);
                self.pos_false.fix();
            }
        }
    }

    fn is_finished_minimal(&self) -> bool {
        self.pos_false.is_fixed() || self.pos_true.is_fixed()
    }

    fn is_finished(&self) -> bool {
        self.pos_false.is_fixed() && self.pos_true.is_fixed()
    }

    fn results(&self) -> (Option<usize>, Option<usize>) {
        (self.pos_false.into(), self.pos_true.into())
    }
}

/// A brute-force implementation of [`cyclic_breadth_binary_search`].
fn cbps_brute_force<EF>(
    bounds: core::ops::Range<usize>,
    eval: &EF,
) -> (Option<usize>, Option<usize>)
where
    EF: Fn(usize) -> bool,
{
    let mut discover = Discover::new();

    for i in bounds {
        discover.update(i, eval(i));
        if discover.is_finished() {
            break;
        }
    }

    discover.results()
}

/// Search for the largest index inside the bounds which still fulfills the condition
fn binary_search<T, EF>(
    eval: &EF,
    expected_value: T,
    mut bounds: core::ops::Range<usize>,
) -> Option<usize>
where
    EF: Fn(usize) -> T,
    T: Eq + core::fmt::Debug,
{
    assert!(bounds.start <= bounds.end);
    if bounds.is_empty() || eval(bounds.start) != expected_value {
        return None;
    }

    while bounds.start + 1 < bounds.end {
        let middle = bounds.start + (bounds.end - bounds.start) / 2;
        debug_assert_ne!(middle, bounds.start);
        debug_assert_ne!(middle, bounds.end);

        // binary search for partition bounds
        if eval(middle) == expected_value {
            bounds.start = middle;
        } else {
            bounds.end = middle;
        }
    }

    debug_assert_eq!(eval(bounds.start), expected_value);
    Some(bounds.start)
}

/// Perform a breadth-first search on an induced binary tree on the list,
/// searching for the bounds of the partition induced by `eval`,
/// returning the last item indices in the `false` and `true` blocks
pub fn cyclic_breadth_partition_search<EF>(
    bounds: Range<usize>,
    eval: EF,
) -> (Option<usize>, Option<usize>)
where
    EF: Fn(usize) -> bool,
{
    if bounds.is_empty() {
        return (None, None);
    }

    // discover gaps (true is a gap for false and vice versa)
    let mut discover = Discover::new();

    for i in 0..((bounds.end - bounds.start).ilog2() as u8) {
        breadth4level(bounds.clone(), i, |bounds| {
            let middle = bounds.start + (bounds.end - bounds.start) / 2;
            discover.update(middle, eval(middle));
            !discover.is_finished_minimal()
        });
        if discover.is_finished_minimal() {
            break;
        }
    }

    // brute force on failure
    if !discover.is_finished() {
        return cbps_brute_force(bounds, &eval);
    }

    let (pos_false, pos_true) = discover.results();
    let (mut pos_false, mut pos_true) = (pos_false.unwrap(), pos_true.unwrap());

    // discover bounds
    debug_assert_ne!(pos_false, pos_true);

    // whatever block is at the beginning has
    // its end somewhere strictly before the other block
    // either:
    // - the later block continues at the beginning
    //   format: L...!L...L...
    // - or the later block doesn't continue at the beginning
    //   format: !L...L...
    let val_start = eval(bounds.start);

    {
        let (pos_start, pos_next) = match val_start {
            false => (&mut pos_false, &mut pos_true),
            true => (&mut pos_true, &mut pos_false),
        };

        *pos_start = binary_search(&eval, val_start, bounds.start..*pos_next).unwrap();
        *pos_next = binary_search(&eval, !val_start, *pos_start + 1..bounds.end).unwrap();
    }

    (Some(pos_false), Some(pos_true))
}

#[cfg(test)]
mod tests {
    use super::{cbps_brute_force, cyclic_breadth_partition_search as cbps};
    use proptest::prelude::*;

    fn cbps_assert_eq<T, PF>(list: &[T], partition: PF) -> (Option<usize>, Option<usize>)
    where
        PF: Fn(&T) -> bool,
        T: Eq + core::fmt::Debug,
    {
        let eval = &|i: usize| partition(&list[i]);
        let res_expected = cbps_brute_force(0..list.len(), eval);
        assert_eq!(cbps(0..list.len(), eval), res_expected);
        res_expected
    }

    #[test]
    fn cbps_bpw3_simple00() {
        let list = &[false, false, false, true, true];
        assert_eq!(cbps_assert_eq(list, |i| *i), (Some(2), Some(4)));
    }

    #[test]
    fn cbps_bpw3_cont_false() {
        let list = &[false, false, false];
        assert_eq!(cbps_assert_eq(list, |i| *i), (Some(2), None));
    }

    #[test]
    fn cbps_bpw3_cont_true() {
        let list = &[true, true];
        assert_eq!(cbps_assert_eq(list, |i| *i), (None, Some(1)));
    }

    #[test]
    fn cbps_bpw3_simple01() {
        let list = &[true, false, false, false, true, true];
        assert_eq!(cbps_assert_eq(list, |i| *i), (Some(3), Some(0)));
    }

    #[test]
    fn cbps_bpw3_1exception() {
        let list = &[true, false, true, true, true, true];
        assert_eq!(cbps_assert_eq(list, |i| *i), (Some(1), Some(0)));
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 4096, .. ProptestConfig::default()
        })]

        #[test]
        fn cbps_arbitrary(len in 1..4096usize, offset in 0..4096usize, amount_true in 0..4096usize) {
            let offset = offset % len;
            let amount_true = amount_true % len;

            let mut list = vec![false; len];
            for i in offset..(offset + amount_true) {
                let i = i % len;
                list[i] = true;
            }
            let eval = &|i: usize| list[i];
            prop_assert_eq!(
                cbps(0..list.len(), eval),
                cbps_brute_force(0..list.len(), eval)
            );
        }
    }
}
