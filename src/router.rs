use geo::geometry::Point;
use petgraph::visit::EdgeRef;
use spade::InsertionError;

use crate::astar::{astar, AstarStrategy, PathTracker};
use crate::graph::{DotIndex, FixedDotIndex};
use crate::layout::Layout;

use crate::mesh::{Mesh, MeshEdgeReference, VertexIndex};

use crate::rules::Rules;
use crate::tracer::{Trace, Tracer};

pub trait RouterObserver {
    fn on_rework(&mut self, tracer: &Tracer, trace: &Trace);
    fn before_probe(&mut self, tracer: &Tracer, trace: &Trace, edge: MeshEdgeReference);
    fn on_probe(&mut self, tracer: &Tracer, trace: &Trace, edge: MeshEdgeReference);
    fn on_estimate(&mut self, tracer: &Tracer, vertex: VertexIndex);
}

pub struct Router {
    pub layout: Layout,
    rules: Rules,
}

struct RouterAstarStrategy<'a, RO: RouterObserver> {
    tracer: Tracer<'a>,
    trace: Trace,
    to: VertexIndex,
    observer: &'a mut RO,
}

impl<'a, RO: RouterObserver> RouterAstarStrategy<'a, RO> {
    pub fn new(tracer: Tracer<'a>, trace: Trace, to: VertexIndex, observer: &'a mut RO) -> Self {
        Self {
            tracer,
            trace,
            to,
            observer,
        }
    }
}

impl<'a, RO: RouterObserver> AstarStrategy<&Mesh, u64> for RouterAstarStrategy<'a, RO> {
    fn is_goal(&mut self, vertex: VertexIndex, tracker: &PathTracker<&Mesh>) -> bool {
        let new_path = tracker.reconstruct_path_to(vertex);

        self.tracer.rework_path(&mut self.trace, &new_path, 5.0);
        self.observer.on_rework(&self.tracer, &self.trace);

        self.tracer
            .finish(&mut self.trace, self.tracer.mesh.dot(self.to), 5.0)
            .is_ok()
    }

    fn edge_cost(&mut self, edge: MeshEdgeReference) -> Option<u64> {
        self.observer.before_probe(&self.tracer, &self.trace, edge);
        if edge.target() != self.to
            && self
                .tracer
                .step(&mut self.trace, edge.target(), 5.0)
                .is_ok()
        {
            self.observer.on_probe(&self.tracer, &self.trace, edge);
            self.tracer.undo_step(&mut self.trace);
            Some(1)
        } else {
            None
        }
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> u64 {
        self.observer.on_estimate(&self.tracer, vertex);
        0
    }
}

impl Router {
    pub fn new() -> Self {
        Router {
            layout: Layout::new(),
            rules: Rules::new(),
        }
    }

    pub fn enroute(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        observer: &mut impl RouterObserver,
    ) -> Result<Mesh, InsertionError> {
        // XXX: Should we actually store the mesh? May be useful for debugging, but doesn't look
        // right.
        //self.mesh.triangulate(&self.layout)?;
        let mut mesh = Mesh::new();
        mesh.triangulate(&self.layout)?;

        let mut tracer = self.tracer(&mesh);
        let trace = tracer.start(from);

        let (_cost, _path) = astar(
            &mesh,
            mesh.vertex(from),
            &mut RouterAstarStrategy::new(tracer, trace, mesh.vertex(to), observer),
        )
        .unwrap(); // TODO.

        Ok(mesh)
    }

    pub fn reroute(
        &mut self,
        _from: FixedDotIndex,
        _to: Point,
        _observer: &mut impl RouterObserver,
    ) -> Result<Mesh, InsertionError> {
        /*let to_dot = if let Some(band) = self.layout.next_band(from) {
            let to_dot = band.ends().1;

            self.layout.remove_interior(&band);
            self.layout.move_dot(to_dot, to);
            to_dot
        } else {
            let from_weight = self.layout.primitive(from).weight();
            self.layout
                .add_fixed_dot(FixedDotWeight {
                    net: from_weight.net,
                    circle: Circle { pos: to, r: 2.0 },
                })
                .unwrap() // TODO.
        };

        self.enroute(from, to_dot, observer)*/
        Ok(Mesh::new())
    }

    /*pub fn squeeze_around_dot(
        &mut self,
        head: Head,
        around: FixedDotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw().segbend_around_dot(head, around, cw, width)?;
        self.layout
            .reattach_bend(outer, head.segbend.as_ref().unwrap().bend);

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn squeeze_around_bend(
        &mut self,
        head: Head,
        around: FixedBendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw().segbend_around_bend(head, around, cw, width)?;
        self.layout
            .reattach_bend(outer, head.segbend.as_ref().unwrap().bend);

        self.reroute_outward(outer)?;
        Ok(head)
    }

    fn reroute_outward(&mut self, bend: FixedBendIndex) -> Result<(), ()> {
        let mut bows: Vec<Bow> = vec![];
        let cw = self.layout.primitive(bend).weight().cw;

        let mut cur_bend = bend;
        loop {
            bows.push(self.layout.bow(cur_bend));

            cur_bend = match self.layout.primitive(cur_bend).outer() {
                Some(new_bend) => new_bend,
                None => break,
            }
        }

        let core = self.layout.primitive(bend).core().unwrap();
        let mut maybe_inner = self.layout.primitive(bend).inner();

        for bow in &bows {
            self.layout.remove_interior(bow);
        }

        for bow in &bows {
            let ends = bow.ends();
            let mut head = self.draw().start(ends.0);
            let width = 5.0;

            if let Some(inner) = maybe_inner {
                head = self.draw().segbend_around_bend(head, inner, cw, width)?;
            } else {
                head = self.draw().segbend_around_dot(head, core, cw, width)?;
            }

            maybe_inner = head.segbend.as_ref().map(|segbend| segbend.bend);
            self.draw().finish(head, ends.1, width)?;
            self.relax_band(maybe_inner.unwrap());
        }

        Ok(())
    }

    fn relax_band(&mut self, bend: FixedBendIndex) {
        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).find_prev_akin() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }

        let mut prev_bend = bend;
        while let Some(cur_bend) = self.layout.primitive(prev_bend).find_next_akin() {
            if self.layout.primitive(cur_bend).cross_product() >= 0. {
                self.release_bow(cur_bend);
            }

            prev_bend = cur_bend;
        }
    }

    fn release_bow(&mut self, bend: FixedBendIndex) {
        let bow = self.layout.bow(bend);
        let ends = bow.ends();

        self.layout.remove_interior(&bow);

        let head = self.draw().start(ends.0);
        let _ = self.draw().finish(head, ends.1, 5.0);
    }

    pub fn move_dot(&mut self, dot: FixedDotIndex, to: Point) -> Result<(), ()> {
        self.layout.move_dot(dot, to)?;

        if let Some(outer) = self.layout.primitive(dot).outer() {
            self.reroute_outward(outer)?;
        }

        Ok(())
    }*/

    pub fn tracer<'a>(&'a mut self, mesh: &'a Mesh) -> Tracer {
        Tracer::new(&mut self.layout, &self.rules, mesh)
    }

    /*pub fn routeedges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.mesh.edge_references().map(|edge| {
            (
                self.mesh.position(edge.source()),
                self.mesh.position(edge.target()),
            )
        })
    }*/
}
