use geo::Point;
use petgraph::stable_graph::{EdgeIndex, NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use rstar::primitives::GeomWithData;
use rstar::RTree;
use spade::{DelaunayTriangulation, HasPosition, Point2, Triangulation};

use crate::bow::Bow;
use crate::graph::{
    BendIndex, BendWeight, DotIndex, DotWeight, Index, Label, Path, SegIndex, SegWeight, Tag,
    TaggedIndex, TaggedWeight,
};
use crate::primitive::Primitive;
use crate::shape::Shape;

pub type RTreeWrapper = GeomWithData<Shape, TaggedIndex>;

pub struct Layout {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<TaggedWeight, Label, usize>,
}

impl Layout {
    pub fn new() -> Self {
        Layout {
            rtree: RTree::new(),
            graph: StableDiGraph::default(),
        }
    }

    pub fn remove_open_set(&mut self, open_set: Vec<TaggedIndex>) {
        for index in open_set.iter().filter(|index| !index.is_dot()) {
            untag!(index, self.remove(*index));
        }

        // We must remove the dots only after the segs and bends because we need dots to calculate
        // the shapes, which we need to remove the segs and bends from the R-tree.

        for index in open_set.iter().filter(|index| index.is_dot()) {
            untag!(index, self.remove(*index));
        }
    }

    pub fn remove<Weight: std::marker::Copy>(&mut self, index: Index<Weight>) {
        // Unnecessary retag. It should be possible to elide it.
        let weight = *self.graph.node_weight(index.index).unwrap();

        let wrapper = RTreeWrapper::new(self.primitive(index).shape(), index.retag(&weight));
        assert!(self.rtree.remove(&wrapper).is_some());
        self.graph.remove_node(index.index);
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> Result<DotIndex, ()> {
        let dot = DotIndex::new(self.graph.add_node(TaggedWeight::Dot(weight)));

        self.fail_and_remove_if_collides_except(dot, &[])?;
        self.insert_into_rtree(dot.tag());

        Ok(dot)
    }

    pub fn add_seg(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        weight: SegWeight,
    ) -> Result<SegIndex, ()> {
        let seg = SegIndex::new(self.graph.add_node(TaggedWeight::Seg(weight)));

        self.graph.add_edge(from.index, seg.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        self.fail_and_remove_if_collides_except(seg, &[from.tag(), to.tag()])?;
        self.insert_into_rtree(seg.tag());

        self.graph
            .node_weight_mut(from.index)
            .unwrap()
            .as_dot_mut()
            .unwrap()
            .net = weight.net;
        self.graph
            .node_weight_mut(to.index)
            .unwrap()
            .as_dot_mut()
            .unwrap()
            .net = weight.net;

        Ok(seg)
    }

    pub fn add_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        around: TaggedIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        match around {
            TaggedIndex::Dot(core) => self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) => self.add_outer_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
        }
    }

    pub fn add_core_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        core: DotIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        let bend = BendIndex::new(self.graph.add_node(TaggedWeight::Bend(weight)));

        self.graph.add_edge(from.index, bend.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        self.fail_and_remove_if_collides_except(bend, &[from.tag(), to.tag(), core.tag()])?;
        self.insert_into_rtree(bend.tag());
        Ok(bend)
    }

    pub fn add_outer_bend(
        &mut self,
        from: DotIndex,
        to: DotIndex,
        inner: BendIndex,
        weight: BendWeight,
    ) -> Result<BendIndex, ()> {
        let core = *self
            .graph
            .neighbors(inner.index)
            .filter(|ni| {
                self.graph
                    .edge_weight(self.graph.find_edge(inner.index, *ni).unwrap())
                    .unwrap()
                    .is_core()
            })
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();
        let bend = self.add_core_bend(from, to, core, weight)?;
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
        Ok(bend)
    }

    pub fn reattach_bend(&mut self, bend: BendIndex, inner: BendIndex) {
        if let Some(old_inner_edge) = self
            .graph
            .edges_directed(bend.index, Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
    }

    pub fn extend_bend(&mut self, bend: BendIndex, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.remove_from_rtree(bend.tag());
        self.move_dot(dot, to)?;
        self.insert_into_rtree(bend.tag());
        Ok(())
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        let mut cur_bend = self.primitive(dot).outer();
        loop {
            match cur_bend {
                Some(..) => (),
                None => break,
            }

            self.remove_from_rtree(cur_bend.unwrap().tag());
            cur_bend = self.primitive(cur_bend.unwrap()).outer();
        }

        self.remove_from_rtree(dot.tag());

        let mut dot_weight = self.primitive(dot).weight();
        let old_weight = dot_weight;

        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.index).unwrap() = TaggedWeight::Dot(dot_weight);

        if let Some(..) = self.detect_collision_except(dot, &[]) {
            // Restore original state.
            *self.graph.node_weight_mut(dot.index).unwrap() = TaggedWeight::Dot(old_weight);
            self.insert_into_rtree(dot.tag());

            return Err(());
        }

        self.insert_into_rtree(dot.tag());

        let mut cur_bend = self.primitive(dot).outer();
        loop {
            match cur_bend {
                Some(..) => (),
                None => break,
            }

            self.insert_into_rtree(cur_bend.unwrap().tag());
            cur_bend = self.primitive(cur_bend.unwrap()).outer();
        }

        Ok(())
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    pub fn bow(&self, bend: BendIndex) -> Bow {
        Bow::new(bend, &self.graph)
    }

    /*fn triangulate(&mut self) {
        let peer_edge_indices: Vec<EdgeIndex<usize>> = self
            .graph
            .edge_indices()
            .filter(|index| *self.graph.edge_weight(*index).unwrap() == Label::Peer)
            .collect();

        for edge_index in peer_edge_indices {
            self.graph.remove_edge(edge_index);
        }

        struct TriangulationVertex {
            pub index: NodeIndex<usize>,
            x: f64,
            y: f64,
        }

        impl HasPosition for TriangulationVertex {
            type Scalar = f64;
            fn position(&self) -> Point2<Self::Scalar> {
                Point2::new(self.x, self.y)
            }
        }

        let mut triangulation: DelaunayTriangulation<_> = DelaunayTriangulation::new();

        for node_index in self
            .graph
            .node_indices()
            .filter(|index| self.graph.node_weight(*index).unwrap().is_dot())
        {
            let center = self
                .primitive(Index::<Label>::new(node_index))
                .shape()
                .center();
            triangulation
                .insert(TriangulationVertex {
                    index: node_index,
                    x: center.x(),
                    y: center.y(),
                })
                .unwrap();
        }

        for edge in triangulation.directed_edges() {
            let from = edge.from().as_ref().index;
            let to = edge.to().as_ref().index;
            self.graph.add_edge(from, to, Label::Peer);
        }
    }*/

    fn fail_and_remove_if_collides_except<Weight: std::marker::Copy>(
        &mut self,
        index: Index<Weight>,
        except: &[TaggedIndex],
    ) -> Result<(), ()> {
        if let Some(..) = self.detect_collision_except(index, except) {
            self.remove(index);
            return Err(());
        }
        Ok(())
    }

    fn detect_collision_except<Weight: std::marker::Copy>(
        &self,
        index: Index<Weight>,
        except: &[TaggedIndex],
    ) -> Option<TaggedIndex> {
        let primitive = self.primitive(index);
        let shape = primitive.shape();

        self.rtree
            .locate_in_envelope_intersecting(&shape.envelope())
            .filter(|wrapper| {
                let index = wrapper.data;
                !untag!(index, primitive.connectable(index))
            })
            .filter(|wrapper| !except.contains(&wrapper.data))
            .filter(|wrapper| shape.intersects(wrapper.geom()))
            .map(|wrapper| wrapper.data)
            .next()
    }

    fn insert_into_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    fn remove_from_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.remove(&RTreeWrapper::new(shape, index));
    }

    /*pub fn edges(&self) -> impl Iterator<Item = (TaggedIndex, TaggedIndex)> + '_ {
        self.graph.edge_indices().map(|edge| {
            let endpoints = self.graph.edge_endpoints(edge).unwrap();
            (
                Index::<Label>::new(endpoints.0)
                    .retag(self.graph.node_weight(endpoints.0).unwrap()),
                Index::<Label>::new(endpoints.1)
                    .retag(self.graph.node_weight(endpoints.1).unwrap()),
            )
        })
    }*/

    pub fn dots(&self) -> impl Iterator<Item = DotIndex> + '_ {
        self.nodes()
            .filter_map(|ni| ni.as_dot().and_then(|di| Some(*di)))
    }

    pub fn shapes(&self) -> impl Iterator<Item = Shape> + '_ {
        self.nodes()
            .map(|ni| untag!(ni, self.primitive(ni).shape()))
    }

    fn nodes(&self) -> impl Iterator<Item = TaggedIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }
}
