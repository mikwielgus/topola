// SPDX-FileCopyrightText: 2025 Topola contributors
//
// SPDX-License-Identifier: MIT
//
//! WIP implementation of incrementally finding planar graph embeddings modulo homotopy equivalence with fixed vertex positions
//!
//! ## usually used generic parameter names
//! * `EP`: type of etched path descriptor
//! * `NI`: type of node indices
//! * `PNI`: type of primal node indices
//! * `Scalar`: type of coordinates of points and vectors (distances)

#![allow(clippy::type_complexity)]
#![no_std]

extern crate alloc;

pub mod algo;
pub mod math;
pub mod mayrev;
pub mod navmesh;
pub mod planarr;
mod utils;

use alloc::boxed::Box;
use core::fmt;

/// Immutable data associated to a topological navmesh edge
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Edge<PNI> {
    pub lhs: Option<PNI>,
    pub rhs: Option<PNI>,
}

impl<PNI> Edge<PNI> {
    #[inline]
    pub fn flip(&mut self) {
        core::mem::swap(&mut self.lhs, &mut self.rhs);
    }

    #[inline]
    pub fn as_ref(&self) -> Edge<&PNI> {
        Edge {
            lhs: self.lhs.as_ref(),
            rhs: self.rhs.as_ref(),
        }
    }
}

impl<PNI: Clone> Edge<&PNI> {
    #[inline]
    pub fn to_owned(&self) -> Edge<PNI> {
        Edge {
            lhs: self.lhs.cloned(),
            rhs: self.rhs.cloned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(
    any(test, feature = "serde"),
    derive(serde::Deserialize, serde::Serialize)
)]
pub enum RelaxedPath<EP, CT> {
    Normal(EP),
    Weak(CT),
}

pub trait GetIndex {
    type Idx: Clone + Eq;

    /// extract the index of the underlying node
    fn get_index(&self) -> Self::Idx;
}

/// Trait as container for navmesh type-level parameters
// in order to avoid extremely long type signatures
pub trait NavmeshBase {
    type PrimalNodeIndex: Clone + Eq + Ord + fmt::Debug;

    /// type for `RelaxedPath::Normal` component
    type EtchedPath: Clone + Eq + fmt::Debug;

    /// type for `RelaxedPath::Weak` component
    type GapComment: Clone + fmt::Debug;

    type Scalar: Clone + fmt::Debug;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum DualIndex {
    /// refers to a Delaunay face
    Inner(usize),
    /// refers to a Delaunay edge
    Outer(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NavmeshIndex<PNI> {
    Dual(DualIndex),
    Primal(PNI),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Node<PNI, Scalar> {
    pub neighs: Box<[NavmeshIndex<PNI>]>,
    pub pos: spade::Point2<Scalar>,
    /// if this node is `DualOuter`, then it has an `open_direction`
    pub open_direction: Option<spade::Point2<Scalar>>,
}

impl<PNI, Scalar> Node<PNI, Scalar> {
    pub fn dual_neighbors(&self) -> impl Iterator<Item = &DualIndex> {
        self.neighs.iter().filter_map(|i| match i {
            NavmeshIndex::Dual(dual) => Some(dual),
            NavmeshIndex::Primal(_) => None,
        })
    }

    pub fn primal_neighbors(&self) -> impl Iterator<Item = &PNI> {
        self.neighs.iter().filter_map(|i| match i {
            NavmeshIndex::Dual(_) => None,
            NavmeshIndex::Primal(prim) => Some(prim),
        })
    }
}
