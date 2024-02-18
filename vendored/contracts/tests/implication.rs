/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use contracts::*;

#[cfg(feature = "mirai_assertions")]
mod mirai_assertion_mocks;

#[test]
fn test_ret_implication() {
    #[ensures(do_thing -> ret.is_some(), "do_thing should cause a Some(_)")]
    #[ensures(!do_thing -> ret.is_none(), "!do_thing should cause a None")]
    fn perform_thing(do_thing: bool) -> Option<usize> {
        if do_thing {
            Some(12)
        } else {
            None
        }
    }

    perform_thing(true);
    perform_thing(false);
}

#[test]
fn test_ret_implication_old() {
    #[ensures(old(*x) % 2 == 0 -> *x % 2 == 0)]
    #[ensures(old(*x) % 2 == 1 -> *x % 2 == 1)]
    fn incr(x: &mut usize) {
        *x += 2;
    }

    let mut x = 0;
    incr(&mut x);

    let mut x = 1;
    incr(&mut x);
}

#[test]
fn test_requires_implication() {
    #[requires(!negative -> value >= 0)]
    #[requires(negative -> value < 0)]
    fn thing(negative: bool, value: isize) {}

    thing(true, -123);

    thing(false, 123);
}

#[test]
#[should_panic(expected = "Post")]
fn test_failing_implication() {
    #[ensures(t -> ret)]
    #[allow(unused_variables)]
    fn only_true(t: bool) -> bool {
        false // oops
    }

    only_true(true);
}

#[test]
fn test_nested_implication() {
    #[ensures(a -> b -> ret.is_some())]
    fn test(a: bool, b: bool) -> Option<usize> {
        if a {
            if b {
                Some(9)
            } else {
                Some(3)
            }
        } else {
            None
        }
    }

    test(true, false);
    test(false, true);
    test(true, true);
}
