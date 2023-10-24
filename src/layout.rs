use contracts::debug_invariant;
use geo::Point;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::{RTree, RTreeObject};

use crate::bow::Bow;
use crate::graph::{
    FixedBendIndex, FixedBendWeight, FixedDotIndex, FixedDotWeight, FixedSegIndex, FixedSegWeight,
    GenericIndex, GetNodeIndex, HalfLooseSegWeight, Index, Interior, Label, LooseDotIndex,
    LooseDotWeight, MakePrimitive, Retag, Weight,
};
use crate::primitive::{GenericPrimitive, GetConnectable, GetWeight, MakeShape};
use crate::segbend::Segbend;
use crate::shape::{Shape, ShapeTrait};

pub type RTreeWrapper = GeomWithData<Shape, Index>;

pub struct Layout {
    rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<Weight, Label, usize>,
}

#[debug_invariant(self.graph.node_count() == self.rtree.size())]
#[debug_invariant(self.test_envelopes())]
impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() - path.interior().len()))]
    pub fn remove_interior(&mut self, path: &impl Interior<Index>) {
        for index in path
            .interior()
            .into_iter()
            .filter(|index| !index.is_fixed_dot())
        {
            self.remove(index);
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we need to remove the segs and bends from the R-tree.

        for index in path
            .interior()
            .into_iter()
            .filter(|index| index.is_fixed_dot())
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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    pub fn add_fixed_dot(&mut self, weight: FixedDotWeight) -> Result<FixedDotIndex, ()> {
        let dot = FixedDotIndex::new(self.graph.add_node(weight.into()));

        self.insert_into_rtree(dot.into());
        self.fail_and_remove_if_collides_except(dot.into(), &[])?;

        Ok(dot)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    pub fn add_loose_dot(&mut self, weight: LooseDotWeight) -> Result<LooseDotIndex, ()> {
        let dot = LooseDotIndex::new(self.graph.add_node(weight.into()));

        self.insert_into_rtree(dot.into());
        self.fail_and_remove_if_collides_except(dot.into(), &[])?;

        Ok(dot)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    pub fn add_fixed_seg(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        weight: FixedSegWeight,
    ) -> Result<FixedSegIndex, ()> {
        let seg = FixedSegIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::Adjacent);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::Adjacent);

        self.insert_into_rtree(seg.into());
        self.fail_and_remove_if_collides_except(seg.into(), &[])?;

        self.graph
            .node_weight_mut(from.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;

        Ok(seg)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    pub fn add_half_loose_seg(
        &mut self,
        from: FixedDotIndex,
        to: LooseDotIndex,
        weight: HalfLooseSegWeight,
    ) -> Result<FixedSegIndex, ()> {
        let seg = FixedSegIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::Adjacent);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::Adjacent);

        self.insert_into_rtree(seg.into());
        self.fail_and_remove_if_collides_except(seg.into(), &[])?;

        self.graph
            .node_weight_mut(from.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;

        Ok(seg)
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count() + 2))]
    pub fn add_loose_seg(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
        weight: HalfLooseSegWeight,
    ) -> Result<FixedSegIndex, ()> {
        let seg = FixedSegIndex::new(self.graph.add_node(weight.into()));

        self.graph
            .add_edge(from.node_index(), seg.node_index(), Label::Adjacent);
        self.graph
            .add_edge(seg.node_index(), to.node_index(), Label::Adjacent);

        self.insert_into_rtree(seg.into());
        self.fail_and_remove_if_collides_except(seg.into(), &[])?;

        self.graph
            .node_weight_mut(from.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.node_index())
            .unwrap()
            .as_fixed_dot_mut()
            .unwrap()
            .net = weight.net;

        Ok(seg)
    }

    #[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
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
    }

    /*#[debug_ensures(ret.is_ok() -> self.graph.node_count() == old(self.graph.node_count() + 1))]
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    pub fn add_loose_bend(
        &mut self,
        from: LooseDotIndex,
        to: LooseDotIndex,
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
    #[debug_ensures(ret.is_err() -> self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(ret.is_ok() -> self.graph.edge_count() == old(self.graph.edge_count() + 3))]
    #[debug_ensures(ret.is_err() -> self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn add_core_bend(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        core: FixedDotIndex,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        let bend = FixedBendIndex::new(self.graph.add_node(weight.into()));

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
    pub fn add_outer_bend(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        inner: FixedBendIndex,
        weight: FixedBendWeight,
    ) -> Result<FixedBendIndex, ()> {
        let core = *self
            .graph
            .neighbors(inner.node_index())
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(inner.node_index(), *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| FixedDotIndex::new(ni))
            .collect::<Vec<FixedDotIndex>>()
            .first()
            .unwrap();

        let bend = FixedBendIndex::new(self.graph.add_node(weight.into()));

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

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count())
        || self.graph.edge_count() == old(self.graph.edge_count() + 1))]
    pub fn reattach_bend(&mut self, bend: FixedBendIndex, inner: FixedBendIndex) {
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
        let cw = self
            .graph
            .node_weight(bend.node_index())
            .unwrap()
            .into_fixed_bend()
            .unwrap()
            .cw;
        self.graph
            .node_weight_mut(bend.node_index())
            .unwrap()
            .as_fixed_bend_mut()
            .unwrap()
            .cw = !cw;
        self.insert_into_rtree(bend.into());
    }

    pub fn bow(&self, bend: FixedBendIndex) -> Bow {
        Bow::from_bend(bend, &self.graph)
    }

    pub fn segbend(&self, dot: FixedDotIndex) -> Option<Segbend> {
        Segbend::from_dot(dot, &self.graph)
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

    pub fn dots(&self) -> impl Iterator<Item = FixedDotIndex> + '_ {
        self.nodes()
            .filter_map(|ni| ni.as_fixed_dot().map(|di| *di))
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes().map(|ni| ni.primitive(&self.graph).shape())
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn nodes(&self) -> impl Iterator<Item = Index> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }
}

#[debug_invariant(self.test_envelopes())]
impl Layout {
    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    pub fn move_dot(&mut self, dot: FixedDotIndex, to: Point) -> Result<(), ()> {
        self.primitive(dot)
            .seg()
            .map(|seg| self.remove_from_rtree(seg.into()));
        self.primitive(dot)
            .bend()
            .map(|bend| self.remove_from_rtree(bend.into()));
        self.remove_from_rtree(dot.into());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::FixedDot(dot_weight);

        if let Some(..) = self.detect_collision_except(dot.into(), &[]) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.node_index()).unwrap() = Weight::FixedDot(old_weight);

            self.insert_into_rtree(dot.into());
            self.primitive(dot)
                .seg()
                .map(|prev| self.insert_into_rtree(prev.into()));
            self.primitive(dot)
                .bend()
                .map(|next| self.insert_into_rtree(next.into()));
            return Err(());
        }

        self.insert_into_rtree(dot.into());
        self.primitive(dot)
            .seg()
            .map(|prev| self.insert_into_rtree(prev.into()));
        self.primitive(dot)
            .bend()
            .map(|next| self.insert_into_rtree(next.into()));

        Ok(())
    }

    pub fn primitive<W>(&self, index: GenericIndex<W>) -> GenericPrimitive<W> {
        GenericPrimitive::new(index, &self.graph)
    }

    fn detect_collision_except(&self, index: Index, except: &[Index]) -> Option<Index> {
        let shape = index.primitive(&self.graph).shape();

        self.rtree
            .locate_in_envelope_intersecting(&RTreeObject::envelope(&shape))
            .filter(|wrapper| {
                let other_index = wrapper.data;
                !index.primitive(&self.graph).connectable(other_index)
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn insert_into_rtree(&mut self, index: Index) {
        let shape = index.primitive(&self.graph).shape();
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    #[debug_ensures(self.graph.node_count() == old(self.graph.node_count()))]
    #[debug_ensures(self.graph.edge_count() == old(self.graph.edge_count()))]
    fn remove_from_rtree(&mut self, index: Index) {
        let shape = index.primitive(&self.graph).shape();
        let removed_element = self.rtree.remove(&RTreeWrapper::new(shape, index));
        debug_assert!(removed_element.is_some());
    }
}

impl Layout {
    fn test_envelopes(&self) -> bool {
        !self.rtree.iter().any(|wrapper| {
            let index = wrapper.data;
            let shape = index.primitive(&self.graph).shape();
            let wrapper = RTreeWrapper::new(shape, index);
            !self
                .rtree
                .locate_in_envelope(&RTreeObject::envelope(&shape))
                .any(|w| *w == wrapper)
        })
    }
}
