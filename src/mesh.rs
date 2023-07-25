use geo::Point;
use petgraph::Direction::Incoming;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use rstar::RTree;
use rstar::primitives::GeomWithData;

use crate::primitive::Primitive;
use crate::shape::Shape;
use crate::graph::{Tag, TaggedIndex, DotIndex, SegIndex, BendIndex, Index, TaggedWeight, DotWeight, SegWeight, BendWeight, Label, Path};
use crate::stretch::Stretch;

pub type RTreeWrapper = GeomWithData<Shape, TaggedIndex>;

pub struct Mesh {
    pub rtree: RTree<RTreeWrapper>,
    pub graph: StableDiGraph<TaggedWeight, Label, usize>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh {
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
        self.rtree.remove(&RTreeWrapper::new(self.primitive(index).shape(), index.retag(weight)));

        self.graph.remove_node(index.index);
    }

    pub fn add_dot(&mut self, weight: DotWeight) -> DotIndex {
        let dot = DotIndex::new(self.graph.add_node(TaggedWeight::Dot(weight)));
        self.rtree.insert(RTreeWrapper::new(self.primitive(dot).shape(), TaggedIndex::Dot(dot)));
        dot
    }

    pub fn add_seg(&mut self, from: DotIndex, to: DotIndex, weight: SegWeight) -> SegIndex {
        let seg = SegIndex::new(self.graph.add_node(TaggedWeight::Seg(weight)));
        self.graph.add_edge(from.index, seg.index, Label::End);
        self.graph.add_edge(seg.index, to.index, Label::End);

        self.rtree.insert(RTreeWrapper::new(self.primitive(seg).shape(), TaggedIndex::Seg(seg)));
        seg
    }

    pub fn add_bend(&mut self, from: DotIndex, to: DotIndex, around: TaggedIndex, weight: BendWeight) -> BendIndex {
        match around {
            TaggedIndex::Dot(core) =>
                self.add_core_bend(from, to, core, weight),
            TaggedIndex::Bend(around) =>
                self.add_outer_bend(from, to, around, weight),
            TaggedIndex::Seg(..) => unreachable!(),
        }
    }

    pub fn add_core_bend(&mut self, from: DotIndex, to: DotIndex, core: DotIndex, weight: BendWeight) -> BendIndex {
        let bend = BendIndex::new(self.graph.add_node(TaggedWeight::Bend(weight)));
        self.graph.add_edge(from.index, bend.index, Label::End);
        self.graph.add_edge(bend.index, to.index, Label::End);
        self.graph.add_edge(bend.index, core.index, Label::Core);

        self.rtree.insert(RTreeWrapper::new(self.primitive(bend).shape(), TaggedIndex::Bend(bend)));
        bend
    }

    pub fn add_outer_bend(&mut self, from: DotIndex, to: DotIndex, inner: BendIndex, weight: BendWeight) -> BendIndex {
        let core = *self.graph.neighbors(inner.index)
            .filter(|ni| self.graph.edge_weight(self.graph.find_edge(inner.index, *ni).unwrap()).unwrap().is_core())
            .map(|ni| DotIndex::new(ni))
            .collect::<Vec<DotIndex>>()
            .first()
            .unwrap();
        let bend = self.add_core_bend(from, to, core, weight);
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
        bend
    }

    pub fn reattach_bend(&mut self, bend: BendIndex, inner: BendIndex) {
        if let Some(old_inner_edge) = self.graph.edges_directed(bend.index, Incoming)
            .filter(|edge| *edge.weight() == Label::Outer)
            .next()
        {
            self.graph.remove_edge(old_inner_edge.id());
        }
        self.graph.add_edge(inner.index, bend.index, Label::Outer);
    }

    pub fn extend_bend(&mut self, bend: BendIndex, dot: DotIndex, to: Point) {
        self.remove_from_rtree(bend.tag());
        self.remove_from_rtree(dot.tag());
        
        let mut dot_weight = self.primitive(dot).weight();
        dot_weight.circle.pos = to;
        *self.graph.node_weight_mut(dot.index).unwrap() = TaggedWeight::Dot(dot_weight);

        self.insert_into_rtree(dot.tag());
        self.insert_into_rtree(bend.tag());
    }

    pub fn nodes(&self) -> impl Iterator<Item=TaggedIndex> + '_ {
        self.rtree.iter().map(|wrapper| wrapper.data)
    }

    pub fn primitive<Weight>(&self, index: Index<Weight>) -> Primitive<Weight> {
        Primitive::new(index, &self.graph)
    }

    pub fn stretch(&self, bend: BendIndex) -> Stretch {
        Stretch::new(bend, &self.graph)
    }

    fn insert_into_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.insert(RTreeWrapper::new(shape, index));
    }

    fn remove_from_rtree(&mut self, index: TaggedIndex) {
        let shape = untag!(index, self.primitive(index).shape());
        self.rtree.remove(&RTreeWrapper::new(shape, index));
    }
}
