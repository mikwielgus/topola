use enum_dispatch::enum_dispatch;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use crate::{
    connectivity::{BandIndex, BandWeight, ComponentWeight, ConnectivityWeight},
    graph::GenericIndex,
    layout::Layout,
    math::Circle,
    primitive::{GenericPrimitive, Primitive},
};

#[enum_dispatch]
pub trait Retag {
    fn retag(&self, index: NodeIndex<usize>) -> Index;
}

#[enum_dispatch]
pub trait GetNet {
    fn net(&self) -> i64;
}

pub trait GetNetMut {
    fn net_mut(&mut self) -> &mut i64;
}

pub trait GetBandIndex {
    fn band(&self) -> BandIndex;
}

#[enum_dispatch]
pub trait GetWidth {
    fn width(&self) -> f64;
}

pub trait GetOffset {
    fn offset(&self) -> f64;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag for $weight_struct {
            fn retag(&self, index: NodeIndex<usize>) -> Index {
                Index::$weight_variant($index_struct::new(index))
            }
        }

        pub type $index_struct = GenericIndex<$weight_struct>;

        impl MakePrimitive for $index_struct {
            fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a> {
                Primitive::$weight_variant(GenericPrimitive::new(*self, layout))
            }
        }
    };
}

macro_rules! impl_fixed_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);

        impl GetNet for $weight_struct {
            fn net(&self) -> i64 {
                self.net
            }
        }

        impl GetNetMut for $weight_struct {
            fn net_mut(&mut self) -> &mut i64 {
                &mut self.net
            }
        }
    };
}

macro_rules! impl_loose_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl_weight!($weight_struct, $weight_variant, $index_struct);

        impl GetBandIndex for $weight_struct {
            fn band(&self) -> BandIndex {
                self.band
            }
        }
    };
}

pub type GeometryGraph = StableDiGraph<GeometryWeight, GeometryLabel, usize>;

#[enum_dispatch(Retag)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryWeight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    LooseSeg(LooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Index {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LooseSeg(LooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DotIndex {
    Fixed(FixedDotIndex),
    Loose(LooseDotIndex),
}

impl From<DotIndex> for Index {
    fn from(dot: DotIndex) -> Self {
        match dot {
            DotIndex::Fixed(fixed) => Index::FixedDot(fixed),
            DotIndex::Loose(loose) => Index::LooseDot(loose),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegIndex {
    Fixed(FixedSegIndex),
    Loose(LooseSegIndex),
}

impl From<SegIndex> for Index {
    fn from(seg: SegIndex) -> Self {
        match seg {
            SegIndex::Fixed(seg) => Index::FixedSeg(seg),
            SegIndex::Loose(seg) => Index::LooseSeg(seg),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BendIndex {
    Fixed(FixedBendIndex),
    Loose(LooseBendIndex),
}

impl From<BendIndex> for Index {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => Index::FixedBend(bend),
            BendIndex::Loose(bend) => Index::LooseBend(bend),
        }
    }
}

impl From<BendIndex> for WraparoundableIndex {
    fn from(bend: BendIndex) -> Self {
        match bend {
            BendIndex::Fixed(bend) => WraparoundableIndex::FixedBend(bend),
            BendIndex::Loose(bend) => WraparoundableIndex::LooseBend(bend),
        }
    }
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WraparoundableIndex {
    FixedDot(FixedDotIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

impl From<WraparoundableIndex> for Index {
    fn from(wraparoundable: WraparoundableIndex) -> Self {
        match wraparoundable {
            WraparoundableIndex::FixedDot(dot) => Index::FixedDot(dot),
            WraparoundableIndex::FixedBend(bend) => Index::FixedBend(bend),
            WraparoundableIndex::LooseBend(bend) => Index::LooseBend(bend),
        }
    }
}
pub trait DotWeight: GetWidth + Into<GeometryWeight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedDotWeight {
    pub net: i64,
    pub circle: Circle,
}

impl_fixed_weight!(FixedDotWeight, FixedDot, FixedDotIndex);
impl DotWeight for FixedDotWeight {}

impl GetWidth for FixedDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseDotWeight {
    pub band: BandIndex,
    pub circle: Circle,
}

impl_loose_weight!(LooseDotWeight, LooseDot, LooseDotIndex);
impl DotWeight for LooseDotWeight {}

impl GetWidth for LooseDotWeight {
    fn width(&self) -> f64 {
        self.circle.r * 2.0
    }
}

pub trait SegWeight: Into<GeometryWeight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedSegWeight {
    pub net: i64,
    pub width: f64,
}

impl_fixed_weight!(FixedSegWeight, FixedSeg, FixedSegIndex);
impl SegWeight for FixedSegWeight {}

impl GetWidth for FixedSegWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseSegWeight {
    pub band: BandIndex,
}

impl_loose_weight!(LooseSegWeight, LooseSeg, LooseSegIndex);
impl SegWeight for LooseSegWeight {}

pub trait BendWeight: Into<GeometryWeight> + Copy {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedBendWeight {
    pub net: i64,
    pub width: f64,
    pub cw: bool,
}

impl_fixed_weight!(FixedBendWeight, FixedBend, FixedBendIndex);
impl BendWeight for FixedBendWeight {}

impl GetWidth for FixedBendWeight {
    fn width(&self) -> f64 {
        self.width
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LooseBendWeight {
    pub band: BandIndex,
    pub offset: f64,
    pub cw: bool,
}

impl GetOffset for LooseBendWeight {
    fn offset(&self) -> f64 {
        self.offset
    }
}

impl_loose_weight!(LooseBendWeight, LooseBend, LooseBendIndex);
impl BendWeight for LooseBendWeight {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryLabel {
    Adjacent,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a>;
}
