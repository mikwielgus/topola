use enum_dispatch::enum_dispatch;

use petgraph::stable_graph::NodeIndex;

use crate::{drawing::Drawing, graph::GetPetgraphIndex};

use super::{
    bend::{FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    primitive::Primitive,
    rules::RulesTrait,
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
        SeqLooseSegWeight,
    },
};

#[enum_dispatch]
pub trait Retag<PrimitiveIndex> {
    fn retag(&self, index: NodeIndex<usize>) -> PrimitiveIndex;
}

#[enum_dispatch]
pub trait GetLayer {
    fn layer(&self) -> usize;
}

#[enum_dispatch]
pub trait GetMaybeNet {
    fn maybe_net(&self) -> Option<usize>;
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a, CW: Copy, R: RulesTrait>(
        &self,
        drawing: &'a Drawing<CW, R>,
    ) -> Primitive<'a, CW, R>;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag<PrimitiveIndex> for $weight_struct {
            fn retag(&self, index: NodeIndex<usize>) -> PrimitiveIndex {
                PrimitiveIndex::$weight_variant($index_struct::new(index))
            }
        }

        impl<'a> GetLayer for $weight_struct {
            fn layer(&self) -> usize {
                self.layer
            }
        }

        impl<'a> GetMaybeNet for $weight_struct {
            fn maybe_net(&self) -> Option<usize> {
                self.maybe_net
            }
        }

        pub type $index_struct = GenericIndex<$weight_struct>;

        impl MakePrimitive for $index_struct {
            fn primitive<'a, CW: Copy, R: RulesTrait>(
                &self,
                drawing: &'a Drawing<CW, R>,
            ) -> Primitive<'a, CW, R> {
                Primitive::$weight_variant(GenericPrimitive::new(*self, drawing))
            }
        }
    };
}

macro_rules! impl_fixed_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);
    };
}

macro_rules! impl_loose_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);
    };
}

// TODO: This enum shouldn't exist: we shouldn't be carrying the tag around like this. Instead we
// should be getting it from the graph when it's needed.
#[enum_dispatch(GetPetgraphIndex, MakePrimitive)]
#[derive(Debug, Hash, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveIndex {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LoneLooseSeg(LoneLooseSegIndex),
    SeqLooseSeg(SeqLooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetWidth, GetLayer, Retag<PrimitiveIndex>)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveWeight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    LoneLooseSeg(LoneLooseSegWeight),
    SeqLooseSeg(SeqLooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}
