/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Testing of simple functions.

use contracts::*;

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[test]
fn test_a_thing() {
    #[requires(x > 10, x < 20, "x must be in valid range")]
    #[ensures(ret > x, "result will be bigger than input")]
    fn a(x: usize) -> usize {
        x + 1
    }

    a(15);
}

#[test]
fn test_sort() {
    fn is_sorted(input: &[usize]) -> bool {
        if input.len() < 2 {
            return true;
        }

        for i in 1..input.len() {
            if input[i - 1] > input[i] {
                return false;
            }
        }

        true
    }

    #[ensures(ret.len() == input.len())]
    #[test_ensures(is_sorted(&ret))]
    fn sort(input: &[usize]) -> Vec<usize> {
        let mut vec = input.to_owned();

        vec.sort_unstable();

        vec
    }

    let input = vec![31, 234, 34, 0, 4234, 85];

    sort(&input);
}

#[test]
fn test_invariant() {
    #[invariant(*val <= 10)]
    fn add_to_10(val: &mut usize) {
        if *val >= 10 {
            return;
        }
        *val += 1;
    }

    let mut val = 8;

    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
    add_to_10(&mut val);
}

#[test]
#[should_panic(expected = "Post-condition of abs violated")]
fn test_early_return() {
    // make sure that post-conditions are executed even if an early return happened.

    #[ensures(ret >= 0)]
    #[ensures(ret == x || ret == -x)]
    #[ensures(ret * ret == x * x)]
    fn abs(x: isize) -> isize {
        if x < 0 {
            // this implementation does not respect the contracts!
            return 0;
        }
        x
    }

    abs(-4);
}

#[test]
fn test_mut_ref_and_lifetimes() {
    #[requires(i < s.len())]
    #[ensures(*ret == 0)]
    fn insert_zero<'a>(s: &'a mut [u8], i: usize) -> &'a mut u8 {
        s[i] = 0;
        &mut s[i]
    }

    insert_zero(&mut [4, 2], 1);
}

#[test]
fn test_pattern_match() {
    #[ensures(ret > a && ret > b)]
    fn add((a, b): (u8, u8)) -> u8 {
        a.saturating_add(b)
    }

    assert_eq!(add((4, 2)), 6);
}

#[test]
fn test_impl_trait_return() {
    // make sure that compiling functions that return existentially
    // qualified types works properly.

    #[requires(x >= 10)]
    #[ensures(Clone::clone(&ret) == ret)]
    #[allow(unused_variables)]
    fn impl_test(x: isize) -> impl Clone + PartialEq + std::fmt::Debug {
        "it worked"
    }

    let x = impl_test(200);
    let y = x.clone();
    assert_eq!(
        format!("{:?} and {:?}", x, y),
        r#""it worked" and "it worked""#
    );
}

#[test]
fn test_impl_trait_arg() {
    #[requires(Clone::clone(&x) == x)]
    #[ensures(Clone::clone(&ret) == ret)]
    fn impl_test(x: impl Clone + PartialEq + std::fmt::Debug) -> &'static str {
        "it worked"
    }

    let x = impl_test(200);
    let y = Clone::clone(&x);
    assert_eq!(
        format!("{:?} and {:?}", x, y),
        r#""it worked" and "it worked""#
    );
}

#[test]
#[deny(clippy::used_underscore_binding)]
fn test_unbound_parameters_clippy() {
    #[requires(__y == 3)]
    #[ensures(ret)]
    fn param_test(_x: i32, __y: i32) -> bool {
        true
    }
}

#[test]
#[deny(non_fmt_panics)]
fn test_braced_condition_expression_clippy() {
    #[requires(if __y == 3 {
        __y != 0
    } else {
        false
    })]
    fn param_test(_x: i32, __y: i32) {}
}
