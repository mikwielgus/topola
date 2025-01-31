// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "(T, T)"))]
#[cfg_attr(
    feature = "serde",
    serde(bound(deserialize = "T: serde::Deserialize<'de> + Ord"))
)]
pub struct OrderedPair<T>(T, T);

impl<T> core::ops::Index<bool> for OrderedPair<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: bool) -> &T {
        if index {
            &self.1
        } else {
            &self.0
        }
    }
}

impl<T: Ord> From<(T, T)> for OrderedPair<T> {
    fn from(mut x: (T, T)) -> Self {
        if x.0 > x.1 {
            core::mem::swap(&mut x.0, &mut x.1);
        }
        let (a, b) = x;
        Self(a, b)
    }
}

impl<T> From<OrderedPair<T>> for (T, T) {
    #[inline(always)]
    fn from(OrderedPair(a, b): OrderedPair<T>) -> (T, T) {
        (a, b)
    }
}

impl<'a, T> From<&'a OrderedPair<T>> for (&'a T, &'a T) {
    #[inline(always)]
    fn from(OrderedPair(a, b): &'a OrderedPair<T>) -> (&'a T, &'a T) {
        (a, b)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for OrderedPair<T> {
    #[inline]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTuple;
        let mut tuple = match serializer.serialize_tuple(2) {
            Ok(x) => x,
            Err(e) => return Err(e),
        };
        match tuple.serialize_element(&self.0) {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
        match tuple.serialize_element(&self.1) {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
        tuple.end()
    }
}
