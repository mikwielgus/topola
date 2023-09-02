use geo::geometry::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;

use crate::astar::{astar, AstarStrategy, PathTracker};
use crate::bow::Bow;
use crate::draw::{Draw, Head};
use crate::graph::{BendIndex, DotIndex, Ends, SegIndex, TaggedIndex};
use crate::graph::{BendWeight, DotWeight, SegWeight};
use crate::guide::Guide;
use crate::layout::Layout;

use crate::math::Circle;
use crate::mesh::{Mesh, MeshEdgeReference, VertexIndex};
use crate::route::{Route, Trace};
use crate::rules::{Conditions, Rules};
use crate::segbend::Segbend;

pub struct Router {
    pub layout: Layout,
    rules: Rules,
}

pub trait RouteStrategy {
    fn route_cost(&mut self, route: &Route, path: &[VertexIndex]) -> u64;
    fn edge_cost(&mut self, route: &Route, edge: MeshEdgeReference) -> u64;
    fn estimate_cost(&mut self, route: &Route, vertex: VertexIndex) -> u64;
}

pub struct DefaultRouteStrategy {}

impl DefaultRouteStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl RouteStrategy for DefaultRouteStrategy {
    fn route_cost(&mut self, route: &Route, path: &[VertexIndex]) -> u64 {
        0
    }

    fn edge_cost(&mut self, route: &Route, edge: MeshEdgeReference) -> u64 {
        1
    }

    fn estimate_cost(&mut self, route: &Route, vertex: VertexIndex) -> u64 {
        0
    }
}

struct RouterAstarStrategy<'a, RS: RouteStrategy> {
    route: Route<'a>,
    trace: Trace,
    to: VertexIndex,
    strategy: RS,
}

impl<'a, RS: RouteStrategy> RouterAstarStrategy<'a, RS> {
    pub fn new(route: Route<'a>, trace: Trace, to: VertexIndex, strategy: RS) -> Self {
        Self {
            route,
            trace,
            to,
            strategy,
        }
    }
}

impl<'a, RS: RouteStrategy> AstarStrategy<&Mesh, u64> for RouterAstarStrategy<'a, RS> {
    fn reroute(&mut self, vertex: VertexIndex, tracker: &PathTracker<&Mesh>) -> Option<u64> {
        let new_path = tracker.reconstruct_path_to(vertex);

        if vertex == self.to {
            self.route
                .rework_path(&mut self.trace, &new_path[..new_path.len() - 1], 5.0)
                .ok();
            self.route
                .finish(&mut self.trace, new_path[new_path.len() - 1], 5.0)
                .ok();
            None
        } else {
            self.route.rework_path(&mut self.trace, &new_path, 5.0).ok();
            Some(self.strategy.route_cost(&self.route, &new_path))
        }
    }

    fn edge_cost(&mut self, edge: MeshEdgeReference) -> u64 {
        self.strategy.edge_cost(&self.route, edge)
    }

    fn estimate_cost(&mut self, vertex: VertexIndex) -> u64 {
        self.strategy.estimate_cost(&self.route, vertex)
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
        from: DotIndex,
        to: DotIndex,
        strategy: impl RouteStrategy,
    ) -> Result<(), InsertionError> {
        // XXX: Should we actually store the mesh? May be useful for debugging, but doesn't look
        // right.
        //self.mesh.triangulate(&self.layout)?;
        let mut mesh = Mesh::new();
        mesh.triangulate(&self.layout)?;

        let mut route = self.route(&mesh);
        let trace = route.start(mesh.vertex(from));

        let (_cost, path) = astar(
            &mesh,
            mesh.vertex(from),
            &mut RouterAstarStrategy::new(route, trace, mesh.vertex(to), strategy),
        )
        .unwrap(); // TODO.
        Ok(())
    }

    /*pub fn squeeze_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
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
        around: BendIndex,
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

    fn reroute_outward(&mut self, bend: BendIndex) -> Result<(), ()> {
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

    fn relax_band(&mut self, bend: BendIndex) {
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

    fn release_bow(&mut self, bend: BendIndex) {
        let bow = self.layout.bow(bend);
        let ends = bow.ends();

        self.layout.remove_interior(&bow);

        let head = self.draw().start(ends.0);
        let _ = self.draw().finish(head, ends.1, 5.0);
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.layout.move_dot(dot, to)?;

        if let Some(outer) = self.layout.primitive(dot).outer() {
            self.reroute_outward(outer)?;
        }

        Ok(())
    }*/

    pub fn route<'a>(&'a mut self, mesh: &'a Mesh) -> Route {
        Route::new(&mut self.layout, &self.rules, mesh)
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
