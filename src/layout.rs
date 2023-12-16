use contracts::debug_invariant;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};
use slab::Slab;

use crate::graph::{
    BendWeight, DotIndex, DotWeight, FixedBendIndex, FixedDotIndex, FixedDotWeight, FixedSegIndex,
    FixedSegWeight, GenericIndex, GetNodeIndex, Index, Interior, Label, LooseBendIndex,
    LooseBendWeight, LooseDotIndex, LooseDotWeight, LooseSegIndex, LooseSegWeight, MakePrimitive,
    Retag, SegWeight, Weight,
};
use crate::primitive::{GenericPrimitive, GetConnectable, GetWeight, MakeShape};
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};

pub type RTreeWrapper = GeomWithData<Shape, Index>;

#[derive(Debug)]
pub struct Band {
    pub net: i64,
    pub width: f64,
}

#[derive(Debug)]
pub struct Layout {
    rtree: RTree<RTreeWrapper>,
    pub bands: Slab<Band>,
    pub graph: StableDiGraph<Weight, Label, usize>,
}

#[debug_invariant(self.graph.node_count() == self.rtree.size())]
#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            bands: Slab::new(),
            graph: StableDiGraph::default(),
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - path.interior().len()))]
    pub fn remove_interior(&mut self, path: &impl Interior<Index>) {
        for index in path
            .interior()
            .into_iter()
            .filter(|index| !matches!(index, Index::LooseDot(..)))
        {
            self.remove(index);
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we need to remove the segs and bends from the R-tree.

        for index in path
            .interior()
            .into_iter()
            .filter(|index| matches!(index, Index::LooseDot(..)))
        {
            self.remove(index);
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - 1))]
    pub fn remove(&mut self, index: Index) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.graph.node_weight(index.node_index()).unwrap();

        self.remove_from_rtree(weight.retag(index.node_index()));
        self.graph.remove_node(index.node_index());
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, ()> {
        self.add_dot(weight)
    }

    // TODO: Remove.
    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_dot(&mut self, weight: LooseDotWeight) -> Result<LooseDotIndex, ()> {
        self.add_dot(weight)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    fn add_dot<W: DotWeight>(&mut self, weight: W) -> Result<GenericIndex<W>, ()>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let dot = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.insert_into_rtree(dot.into());
        self.fail_and_remove_if_collides_except(dot.into(), &[])?;

        Ok(dot)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, ()> {
        self.add_seg(from, to, weight)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 4))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() >= old(self.graph.edge_count() + 5))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_segbend(
        &mut self,
        from: DotIndex,
        around: Index,
        dot_weight: LooseDotWeight,
        seg_weight: LooseSegWeight,
        bend_weight: LooseBendWeight,
    ) -> Result<Segbend, ()> {
        let seg_to = self.add_loose_dot(dot_weight)?;
        let seg = self.add_loose_seg(from, seg_to, seg_weight).map_err(|_| {
            self.remove(seg_to.into());
        })?;

        let bend_to = self.add_loose_dot(dot_weight).map_err(|_| {
            self.remove(seg.into());
            self.remove(seg_to.into());
        })?;
        let bend = self
            .add_loose_bend(seg_to, bend_to, around, bend_weight)
            .map_err(|_| {
                self.remove(bend_to.into());
                self.remove(seg.into());
                self.remove(seg_to.into());
            })?;

        Ok(Segbend {
            seg,
            dot: seg_to,
            bend,
        })
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_loose_seg(
        &mut self,
        from: DotIndex,
        to: LooseDotIndex,
        weight: LooseSegWeight,
    ) -> Result<LooseSegIndex, ()> {
        self.add_seg(from, to, weight)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_seg<W: SegWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        weight: W,
    ) -> Result<GenericIndex<W>, ()>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let seg = GenericIndex::<W>::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::Adjacent);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::Adjacent);

        self.insert_into_rtree(seg.into());
        self.fail_and_remove_if_collides_except(seg.into(), &[])?;

        Ok(seg)
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    pub fn add_fixed_bend(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        around: Index,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        match around {
            Index::FixedDot(core) => self.add_core_bend(from, to, core, weight),
            Index::FixedBend(around) => self.add_outer_bend(from, to, around, weight),
            _ => unreachable!(),
        }
    }*/

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3) || self.graph.edge_count() == old(self.graph.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_loose_bend(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        around: Index,
        weight: LooseBendWeight,
    ) -> Result<LooseBendIndex, ()> {
        match around {
            Index::FixedDot(core) => self.add_core_bend(from, to, core, weight),
            Index::FixedBend(around) => self.add_outer_bend(from, to, around, weight),
            Index::LooseBend(around) => self.add_outer_bend(from, to, around, weight),
            _ => unreachable!(),
        }
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_core_bend<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        core: FixedDotIndex,
        weight: W,
    ) -> Result<LooseBendIndex, ()>
    where
        GenericIndex<W>: Into<Index> + Copy,
    {
        let bend = LooseBendIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_collides_except(bend.into(), &[core.into()])?;
        Ok(bend)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 4))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    fn add_outer_bend<W: BendWeight>(
        &mut self,
        from: impl GetNodeIndex,
        to: impl GetNodeIndex,
        inner: impl GetNodeIndex,
        weight: W,
    ) -> Result<LooseBendIndex, ()> {
        let core = *self
            .graph
            .neighbors(inner.node_index())
            .filter(|ni| {
                matches!(
                    self.graph
                        .edge_weight(self.graph.find_edge(inner.node_index(), *ni).unwrap())
                        .unwrap(),
                    Label::Core
                )
            })
            .map(|ni| FixedDotIndex::new(ni))
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = LooseBendIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), bend.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), to.node_index(), Label::Adjacent);
        self.graph
            .add_edge(bend.node_index(), core.node_index(), Label::Core);
        self.graph
            .add_edge(inner.node_index(), bend.node_index(), Label::Outer);

        self.insert_into_rtree(bend.into());
        self.fail_and_remove_if_collides_except(bend.into(), &[core.into()])?;
        Ok(bend)
    }

    pub fn reposition_bend(&mut self, _bend: LooseBendIndex, _from: Point, _to: Point) {
        // TODO.
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count())
        || self.graph.edge_count() == old(self.graph.edge_count() + 1))]
    pub fn reattach_bend(&mut self, bend: LooseBendIndex, inner: LooseBendIndex) {
        self.remove_from_rtree(bend.into());

        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.node_index(), Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }

        self.graph
            .add_edge(inner.node_index(), bend.node_index(), Label::Outer);
        self.insert_into_rtree(bend.into());
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn flip_bend(&mut self, bend: FixedBendIndex) {
        self.remove_from_rtree(bend.into());

        let Some(Weight::FixedBend(weight)) = self.graph.node_weight_mut(bend.node_index()) else {
            unreachable!();
        };

        weight.cw = !weight.cw;

        self.insert_into_rtree(bend.into());
    }

    /*pub fn bow(&self, bend: LooseBendIndex) -> Bow {
        Bow::from_bend(bend, &self.graph)
    }*/

    pub fn segbend(&self, dot: LooseDotIndex) -> Segbend {
        Segbend::from_dot(dot, self)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count() - 1))]
    fn fail_and_remove_if_collides_except(
        &mut self,
        index: Index,
        except: &[Index],
    ) -> Result<(), ()> {
        if let Some(..) = self.detect_collision_except(index, except) {
            self.remove(index);
            return Err(());
        }
        Ok(())
    }

    pub fn nodes(&self) -> impl Iterator<Item = Index> + '_ {
        self.node_indices().map(|ni| {
            self.graph
                .node_weight(ni.node_index())
                .unwrap()
                .retag(ni.node_index())
        })
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.node_indices().map(|ni| ni.primitive(self).shape())
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn node_indices(&self) -> impl Iterator<Item = Index> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }
}

#[debug_invariant(self.test_envelopes())]
impl Layout {
    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn move_dot(&mut self, dot: LooseDotIndex, to: Point) -> Result<(), ()> {
        self.primitive(dot)
            .seg()
            .map(|seg| self.remove_from_rtree(seg.into()));
        self.remove_from_rtree(self.primitive(dot).bend().into());
        self.remove_from_rtree(dot.into());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::LooseDot(dot_weight);

        if let Some(..) = self.detect_collision_except(dot.into(), &[]) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::LooseDot(old_weight);

            self.insert_into_rtree(dot.into());
            self.primitive(dot)
                .seg()
                .map(|seg| self.remove_from_rtree(seg.into()));
            self.insert_into_rtree(self.primitive(dot).bend().into());
            return Err(());
        }

        self.insert_into_rtree(dot.into());
        self.primitive(dot)
            .seg()
            .map(|seg| self.remove_from_rtree(seg.into()));
        self.insert_into_rtree(self.primitive(dot).bend().into());

        Ok(())
    }

    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W> {
        GenericPrimitive::new(index, self)
    }

    fn detect_collision_except(&self, index: Index, except: &[Index]) -> Option<Index> {
        let shape = index.primitive(self).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !index.primitive(self).connectable(other_index)
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn insert_into_rtree(&mut self, index: Index) {
        let shape = index.primitive(self).shape();
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn remove_from_rtree(&mut self, index: Index) {
        let shape = index.primitive(self).shape();
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, index));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let index = wrapper.data;
            let shape = index.primitive(self).shape();
            let wrapper = RTreeWrapper::new(shape, index);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
