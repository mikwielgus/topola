// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use enum_dispatch::enum_dispatch;

use crate::{
    geometry::GetLayer,
    graph::{GenericIndex, GetIndex},
};

use super::{
    bend::{FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    primitive::PrimitiveRef,
    rules::AccessRules,
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
        SeqLooseSegWeight,
    },
    Drawing,
};

#[enum_dispatch]
pub trait IsInLayer {
    fn is_in_layer(&self, layer: usize) -> bool;

    fn is_in_any_layer_of(&self, layers: &[bool]) -> bool;
}

impl<T: GetLayer> IsInLayer for T {
    #[inline]
    fn is_in_layer(&self, layer: usize) -> bool {
        self.layer() == layer
    }

    fn is_in_any_layer_of(&self, layers: &[bool]) -> bool {
        *layers.get(self.layer()).unwrap_or(&false)
    }
}

#[enum_dispatch]
pub trait GetMaybeNet {
    fn maybe_net(&self) -> Option<usize>;
}

#[enum_dispatch]
pub trait MakePrimitiveRef {
    fn primitive_ref<'a, CW: Clone, Cel: Copy, R: AccessRules>(
        &self,
        drawing: &'a Drawing<CW, Cel, R>,
    ) -> PrimitiveRef<'a, CW, Cel, R>;
}

macro_rules! impl_weight_forward {
    ($weight_struct:ty, $weight_variant:ident, $index_struct:ident) => {
        impl GetLayer for $weight_struct {
            fn layer(&self) -> usize {
                self.0.layer()
            }
        }

        impl GetMaybeNet for $weight_struct {
            fn maybe_net(&self) -> Option<usize> {
                self.0.maybe_net()
            }
        }

        impl GetWidth for $weight_struct {
            fn width(&self) -> f64 {
                self.0.width()
            }
        }

        pub type $index_struct = GenericIndex<$weight_struct>;

        impl MakePrimitiveRef for $index_struct {
            fn primitive_ref<'a, CW: Clone, Cel: Copy, R: AccessRules>(
                &self,
                drawing: &'a crate::drawing::Drawing<CW, Cel, R>,
            ) -> PrimitiveRef<'a, CW, Cel, R> {
                PrimitiveRef::$weight_variant(GenericPrimitive::new(*self, drawing))
            }
        }
    };
}

// TODO: This enum shouldn't exist: we shouldn't be carrying the tag around like this. Instead we
// should be getting it from the graph when it's needed.
#[enum_dispatch(GetIndex, MakePrimitiveRef)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrimitiveIndex {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LoneLooseSeg(LoneLooseSegIndex),
    SeqLooseSeg(SeqLooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetWidth, GetLayer)]
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

impl crate::geometry::Retag for PrimitiveWeight {
    type Index = PrimitiveIndex;

    fn retag(&self, index: usize) -> PrimitiveIndex {
        macro_rules! match_self {
            ($self:expr, $($kind:ident),*,) => {{
                match $self {
                    $(PrimitiveWeight::$kind(_) => PrimitiveIndex::$kind(GenericIndex::new(index))),*
                }
            }}
        }
        match_self!(
            self,
            FixedDot,
            LooseDot,
            FixedSeg,
            LoneLooseSeg,
            SeqLooseSeg,
            FixedBend,
            LooseBend,
        )
    }
}
