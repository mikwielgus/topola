use enum_as_inner::EnumAsInner;
use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use std::marker::PhantomData;

use crate::{
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

pub trait Interior<T> {
    fn interior(&self) -> Vec<T>;
}

pub trait Ends<Start, Stop> {
    fn ends(&self) -> (Start, Stop);
}

#[enum_dispatch]
pub trait Retag {
    fn retag(&self, index: NodeIndex<usize>) -> Index;
}

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

#[enum_dispatch(Retag, GetNet)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Weight {
    Dot(DotWeight),
    Seg(SegWeight),
    Bend(BendWeight),
}

macro_rules! impl_type {
    ($weight_struct_name:ident, $weight_variant_name:ident, $index_struct_name:ident) => {
        impl Retag for $weight_struct_name {
            fn retag(&self, index: NodeIndex<usize>) -> Index {
                Index::$weight_variant_name($index_struct_name {
                    node_index: index,
                    marker: PhantomData,
                })
            }
        }

        impl GetNet for $weight_struct_name {
            fn net(&self) -> i64 {
                self.net
            }
        }

        pub type $index_struct_name = GenericIndex<$weight_struct_name>;

        impl MakePrimitive for $index_struct_name {
            fn primitive<'a>(
                &self,
                graph: &'a StableDiGraph<Weight, Label, usize>,
            ) -> Primitive<'a> {
                Primitive::$weight_variant_name(GenericPrimitive::new(*self, graph))
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_type!(DotWeight, Dot, DotIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegWeight {
    pub net: i64,
    pub width: f64,
}

impl_type!(SegWeight, Seg, SegIndex);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BendWeight {
    pub net: i64,
    pub cw: bool,
}

impl_type!(BendWeight, Bend, BendIndex);

#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Label {
    End,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait GetNodeIndex {
    fn node_index(&self) -> NodeIndex<usize>;
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, graph: &'a StableDiGraph<Weight, Label, usize>) -> Primitive<'a>;
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, EnumAsInner, Clone, Copy, PartialEq)]
pub enum Index {
    Dot(DotIndex),
    Seg(SegIndex),
    Bend(BendIndex),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenericIndex<W> {
    node_index: NodeIndex<usize>,
    marker: PhantomData<W>,
}

impl<W> GenericIndex<W> {
    pub fn new(index: NodeIndex<usize>) -> Self {
        Self {
            node_index: index,
            marker: PhantomData,
        }
    }
}

impl<W> GetNodeIndex for GenericIndex<W> {
    fn node_index(&self) -> NodeIndex<usize> {
        self.node_index
    }
}
