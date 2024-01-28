use std::marker::PhantomData;

use enum_dispatch::enum_dispatch;
use geo::Point;
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    Direction::Incoming,
};

use crate::{
    connectivity::{BandIndex, ComponentIndex},
    graph::{GenericIndex, GetNodeIndex},
    layout::Layout,
    math::Circle,
    primitive::Primitive,
    shape::{BendShape, DotShape, SegShape, Shape},
};

use super::{
    bend::{FixedBendIndex, FixedBendWeight, LooseBendIndex, LooseBendWeight},
    dot::{FixedDotIndex, FixedDotWeight, LooseDotIndex, LooseDotWeight},
    seg::{
        FixedSegIndex, FixedSegWeight, LoneLooseSegIndex, LoneLooseSegWeight, SeqLooseSegIndex,
        SeqLooseSegWeight,
    },
};

#[enum_dispatch]
pub trait Retag {
    fn retag(&self, index: NodeIndex<usize>) -> GeometryIndex;
}

#[enum_dispatch]
pub trait GetComponentIndex {
    fn component(&self) -> ComponentIndex;
}

pub trait GetComponentIndexMut {
    fn component_mut(&mut self) -> &mut ComponentIndex;
}

pub trait GetBandIndex {
    fn band(&self) -> BandIndex;
}

#[enum_dispatch]
pub trait GetPos {
    fn pos(&self) -> Point;
}

#[enum_dispatch]
pub trait GetWidth {
    fn width(&self) -> f64;
}

#[enum_dispatch]
pub trait GetOffset {
    fn offset(&self) -> f64;
}

macro_rules! impl_weight {
    ($weight_struct:ident, $weight_variant:ident, $index_struct:ident) => {
        impl Retag for $weight_struct {
            fn retag(&self, index: NodeIndex<usize>) -> GeometryIndex {
                GeometryIndex::$weight_variant($index_struct::new(index))
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

        impl GetComponentIndex for $weight_struct {
            fn component(&self) -> ComponentIndex {
                self.component
            }
        }

        impl GetComponentIndexMut for $weight_struct {
            fn component_mut(&mut self) -> &mut ComponentIndex {
                &mut self.component
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

#[enum_dispatch(GetWidth, Retag)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryWeight {
    FixedDot(FixedDotWeight),
    LooseDot(LooseDotWeight),
    FixedSeg(FixedSegWeight),
    LoneLooseSeg(LoneLooseSegWeight),
    SeqLooseSeg(SeqLooseSegWeight),
    FixedBend(FixedBendWeight),
    LooseBend(LooseBendWeight),
}

#[enum_dispatch(GetNodeIndex, MakePrimitive)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryIndex {
    FixedDot(FixedDotIndex),
    LooseDot(LooseDotIndex),
    FixedSeg(FixedSegIndex),
    LoneLooseSeg(LoneLooseSegIndex),
    SeqLooseSeg(SeqLooseSegIndex),
    FixedBend(FixedBendIndex),
    LooseBend(LooseBendIndex),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeometryLabel {
    Joint,
    Outer,
    Core,
}

#[enum_dispatch]
pub trait MakePrimitive {
    fn primitive<'a>(&self, layout: &'a Layout) -> Primitive<'a>;
}

pub trait DotWeightTrait<GW>: GetPos + GetWidth + Into<GW> + Copy {}
pub trait SegWeightTrait<GW>: GetWidth + Into<GW> + Copy {}
pub trait BendWeightTrait<GW>: GetOffset + GetWidth + Into<GW> + Copy {}

#[derive(Debug)]
pub struct Geometry<
    GW: GetWidth + TryInto<DW> + TryInto<BW> + Copy,
    DW: DotWeightTrait<GW>,
    BW: BendWeightTrait<GW>,
    DI: GetNodeIndex,
    SI: GetNodeIndex,
    BI: GetNodeIndex,
> {
    pub graph: StableDiGraph<GW, GeometryLabel, usize>,
    weight_marker: PhantomData<GW>,
    dot_weight_marker: PhantomData<DW>,
    bend_weight_marker: PhantomData<BW>,
    dot_index_marker: PhantomData<DI>,
    seg_index_marker: PhantomData<SI>,
    bend_index_marker: PhantomData<BI>,
}

impl<
        GW: GetWidth + TryInto<DW> + TryInto<BW> + Copy,
        DW: DotWeightTrait<GW>,
        BW: BendWeightTrait<GW>,
        DI: GetNodeIndex + Copy,
        SI: GetNodeIndex + Copy,
        BI: GetNodeIndex + Copy,
    > Geometry<GW, DW, BW, DI, SI, BI>
{
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::default(),
            weight_marker: PhantomData,
            dot_weight_marker: PhantomData,
            bend_weight_marker: PhantomData,
            dot_index_marker: PhantomData,
            seg_index_marker: PhantomData,
            bend_index_marker: PhantomData,
        }
    }

    pub fn add_dot<W: DotWeightTrait<GW>>(&mut self, weight: W) -> GenericIndex<W> {
        GenericIndex::<W>::new(self.graph.add_node(weight.into()))
    }

    pub fn add_seg<W: SegWeightTrait<GW>>(
        &mut self,
        from: DI,
        to: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let seg = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .update_edge(from.node_index(), seg.node_index(), GeometryLabel::Joint);
        self.graph
            .update_edge(seg.node_index(), to.node_index(), GeometryLabel::Joint);

        seg
    }

    pub fn add_bend<W: BendWeightTrait<GW>>(
        &mut self,
        from: DI,
        to: DI,
        core: DI,
        weight: W,
    ) -> GenericIndex<W> {
        let bend = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .update_edge(from.node_index(), bend.node_index(), GeometryLabel::Joint);
        self.graph
            .update_edge(bend.node_index(), to.node_index(), GeometryLabel::Joint);
        self.graph
            .update_edge(bend.node_index(), core.node_index(), GeometryLabel::Core);

        bend
    }

    pub fn dot_shape(&self, dot: DI) -> Shape {
        let weight = self.dot_weight(dot);
        Shape::Dot(DotShape {
            c: Circle {
                pos: weight.pos(),
                r: weight.width() / 2.0,
            },
        })
    }

    pub fn seg_shape(&self, seg: SI) -> Shape {
        let joint_weights = self.joint_weights(seg.node_index());
        Shape::Seg(SegShape {
            from: joint_weights[0].pos(),
            to: joint_weights[1].pos(),
            width: self.weight(seg.node_index()).width(),
        })
    }

    pub fn bend_shape(&self, bend: BI) -> Shape {
        let joint_weights = self.joint_weights(bend.node_index());
        let core_weight = self.core_weight(bend);
        Shape::Bend(BendShape {
            from: joint_weights[0].pos(),
            to: joint_weights[1].pos(),
            c: Circle {
                pos: core_weight.pos(),
                r: self.inner_radius(bend),
            },
            width: self.weight(bend.node_index()).width(),
        })
    }

    fn inner_radius(&self, bend: BI) -> f64 {
        let mut r = self.bend_weight(bend).offset();
        let mut rail = bend.node_index();

        while let Some(inner) = self.inner(rail) {
            let weight: BW = self
                .weight(inner)
                .try_into()
                .unwrap_or_else(|_| unreachable!());
            r += weight.width() + weight.offset();
            rail = inner;
        }

        self.core_weight(bend).width() / 2.0 + r
    }

    fn inner(&self, index: NodeIndex<usize>) -> Option<NodeIndex<usize>> {
        self.graph
            .neighbors_directed(index, Incoming)
            .filter(|node| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(*node, index).unwrap())
                        .unwrap(),
                    GeometryLabel::Outer
                )
            })
            .next()
    }

    fn weight(&self, index: NodeIndex<usize>) -> GW {
        *self.graph.node_weight(index).unwrap()
    }

    fn dot_weight(&self, dot: DI) -> DW {
        self.weight(dot.node_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    fn bend_weight(&self, bend: BI) -> BW {
        self.weight(bend.node_index())
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    fn joint_weights(&self, index: NodeIndex<usize>) -> Vec<DW> {
        self.graph
            .neighbors_undirected(index)
            .filter(|node| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge_undirected(index, *node).unwrap().0,)
                        .unwrap(),
                    GeometryLabel::Joint
                )
            })
            .map(|node| {
                self.weight(node)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .collect()
    }

    fn core_weight(&self, bend: BI) -> DW {
        self.graph
            .neighbors(bend.node_index())
            .filter(|node| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(bend.node_index(), *node).unwrap())
                        .unwrap(),
                    GeometryLabel::Core
                )
            })
            .map(|node| {
                self.weight(node)
                    .try_into()
                    .unwrap_or_else(|_| unreachable!())
            })
            .next()
            .unwrap()
    }

    pub fn graph(&self) -> &StableDiGraph<GW, GeometryLabel, usize> {
        &self.graph
    }
}
