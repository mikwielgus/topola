// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! `MaybeReversed`: Structure making it easier to interact with a vector/slice and its reversal uniformly.

use alloc::{sync::Arc, vec::Vec};
use core::{borrow, iter::DoubleEndedIterator, marker::PhantomData, ops};

/// Structure making it easier to interact with a vector/slice and its reversal uniformly.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaybeReversed<I, T: ?Sized> {
    pub inner: I,
    pub reversed: bool,
    item_marker: PhantomData<fn(T) -> T>,
}

impl<I: Clone, T: ?Sized> Clone for MaybeReversed<I, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }
}

impl<I: Copy, T: ?Sized> Copy for MaybeReversed<I, T> {}

impl<I, T> MaybeReversed<I, T> {
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            reversed: false,
            item_marker: PhantomData,
        }
    }

    pub fn map<J, U, F>(self, f: F) -> MaybeReversed<J, U>
    where
        F: FnOnce(I) -> J,
    {
        let Self {
            inner, reversed, ..
        } = self;
        MaybeReversed {
            inner: f(inner),
            reversed,
            item_marker: PhantomData,
        }
    }

    #[inline]
    #[must_use]
    pub fn flip(mut self) -> Self {
        self.reversed ^= true;
        self
    }
}

impl<T> MaybeReversed<&mut Arc<[T]>, T> {
    pub fn with_borrow_mut<R, F>(&mut self, f: F) -> R
    where
        F: FnOnce(MaybeReversed<&mut Vec<T>, T>) -> R,
        T: Clone,
    {
        let mut inner: Vec<T> = self.inner.iter().cloned().collect();
        let ret = f(MaybeReversed {
            inner: &mut inner,
            reversed: self.reversed,
            item_marker: PhantomData,
        });
        *self.inner = Arc::from(inner.into_boxed_slice());
        ret
    }
}

impl<'a, I: ?Sized + borrow::Borrow<[T]>, T> MaybeReversed<&'a I, T> {
    pub fn as_ref(&self) -> MaybeReversed<&'a [T], T> {
        MaybeReversed {
            inner: self.inner.borrow(),
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    pub fn resolve_index(&self, index: usize) -> usize {
        if self.reversed {
            self.len().checked_sub(index + 1).unwrap()
        } else {
            index
        }
    }

    #[inline]
    pub fn iter(&self) -> MaybeReversed<core::slice::Iter<'a, T>, T> {
        MaybeReversed {
            inner: self.inner.borrow().iter(),
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }
}

impl<I: ?Sized + borrow::Borrow<[T]>, T> MaybeReversed<&mut I, T> {
    #[inline(always)]
    pub fn as_ref(&self) -> MaybeReversed<&'_ I, T> {
        MaybeReversed {
            inner: &*self.inner,
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    #[inline]
    pub fn resolve_index(&self, index: usize) -> usize {
        self.as_ref().resolve_index(index)
    }

    #[inline]
    pub fn iter(&self) -> MaybeReversed<core::slice::Iter<'_, T>, T> {
        self.as_ref().iter()
    }
}

impl<T> MaybeReversed<&mut [T], T> {
    #[inline]
    pub fn iter_mut(&mut self) -> MaybeReversed<core::slice::IterMut<'_, T>, T> {
        MaybeReversed {
            inner: self.inner.iter_mut(),
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }
}

impl<T> MaybeReversed<&mut Vec<T>, T> {
    #[inline(always)]
    pub fn as_ref_slice(&self) -> MaybeReversed<&'_ [T], T> {
        MaybeReversed {
            inner: &self.inner[..],
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> MaybeReversed<&'_ mut [T], T> {
        MaybeReversed {
            inner: &mut self.inner[..],
            reversed: self.reversed,
            item_marker: PhantomData,
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        if self.reversed {
            self.inner
                .insert(self.len().checked_sub(index).unwrap(), element);
        } else {
            self.inner.insert(index, element);
        }
    }
}

impl<I: ?Sized + borrow::Borrow<[T]> + ops::Index<usize, Output = T>, T> ops::Index<usize>
    for MaybeReversed<&I, T>
{
    type Output = <I as ops::Index<usize>>::Output;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(self.resolve_index(index))
    }
}

impl<I: ?Sized + borrow::Borrow<[T]> + ops::Index<usize, Output = T>, T> ops::Index<usize>
    for MaybeReversed<&mut I, T>
{
    type Output = <I as ops::Index<usize>>::Output;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.index(self.resolve_index(index))
    }
}

impl<I: ?Sized + borrow::Borrow<[T]> + ops::IndexMut<usize, Output = T>, T> ops::IndexMut<usize>
    for MaybeReversed<&mut I, T>
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner.index_mut(self.resolve_index(index))
    }
}

#[inline]
pub fn index<T>(obj: &[T], idx: MaybeReversed<usize, T>) -> &T {
    use ops::Index;
    obj.index(
        MaybeReversed {
            inner: obj,
            reversed: idx.reversed,
            item_marker: PhantomData,
        }
        .resolve_index(idx.inner),
    )
}

#[inline]
pub fn index_mut<T>(obj: &mut [T], idx: MaybeReversed<usize, T>) -> &mut T {
    use ops::IndexMut;
    obj.index_mut(
        MaybeReversed {
            inner: &obj[..],
            reversed: idx.reversed,
            item_marker: PhantomData,
        }
        .resolve_index(idx.inner),
    )
}

#[inline]
pub fn index_forward<T, U>(obj: &[T], idx: MaybeReversed<usize, U>) -> MaybeReversed<&T, U> {
    MaybeReversed {
        inner: &obj[idx.inner],
        reversed: idx.reversed,
        item_marker: PhantomData,
    }
}

#[inline]
pub fn index_mut_forward<T, U>(
    obj: &mut [T],
    idx: MaybeReversed<usize, U>,
) -> MaybeReversed<&mut T, U> {
    MaybeReversed {
        inner: &mut obj[idx.inner],
        reversed: idx.reversed,
        item_marker: PhantomData,
    }
}

impl<I: DoubleEndedIterator, T> Iterator for MaybeReversed<I, T> {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        match self.reversed {
            false => self.inner.next(),
            true => self.inner.next_back(),
        }
    }

    fn fold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        match self.reversed {
            false => self.inner.fold(init, f),
            true => self.inner.rfold(init, f),
        }
    }
}

impl<I: DoubleEndedIterator, T> DoubleEndedIterator for MaybeReversed<I, T> {
    fn next_back(&mut self) -> Option<<I as Iterator>::Item> {
        match self.reversed {
            false => self.inner.next_back(),
            true => self.inner.next(),
        }
    }

    fn rfold<B, F>(self, init: B, f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        match self.reversed {
            false => self.inner.rfold(init, f),
            true => self.inner.fold(init, f),
        }
    }
}
