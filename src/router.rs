use geo::geometry::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;

use crate::astar::astar;
use crate::bow::Bow;
use crate::draw::Draw;
use crate::graph::{BendIndex, DotIndex, Ends, SegIndex, TaggedIndex};
use crate::graph::{BendWeight, DotWeight, SegWeight};
use crate::guide::Guide;
use crate::layout::Layout;

use crate::math::Circle;
use crate::mesh::{Mesh, VertexIndex};
use crate::rules::{Conditions, Rules};
use crate::segbend::Segbend;

pub struct Router {
    pub layout: Layout,
    mesh: Mesh,
    rules: Rules,
}

struct Route {
    path: Vec<VertexIndex>,
    head: Head,
    width: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Head {
    pub dot: DotIndex,
    pub segbend: Option<Segbend>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            layout: Layout::new(),
            mesh: Mesh::new(),
            rules: Rules::new(),
        }
    }

    pub fn route(&mut self, from: DotIndex, to: DotIndex) -> Result<(), InsertionError> {
        // XXX: Should we actually store the mesh? May be useful for debugging, but doesn't look
        // right.
        self.mesh.triangulate(&self.layout)?;

        let (_cost, mesh_path) = astar(
            &self.mesh,
            self.mesh.vertex(from),
            |node, tracker| {
                let new_path = tracker.reconstruct_path_to(node);

                (node != self.mesh.vertex(to)).then_some(0)
            },
            |_edge| 1,
            |_| 0,
        )
        .unwrap(); // TODO.

        let path: Vec<DotIndex> = mesh_path
            .iter()
            .map(|vertex| self.mesh.dot(*vertex))
            .collect();

        let mut route = self.route_start(path[0], 5.0);
        route = self.route_path(route, &path[1..(path.len() - 1)]).unwrap(); // TODO.
        let _ = self.route_finish(route, path[path.len() - 1]);

        Ok(())
    }

    fn route_start(&mut self, from: DotIndex, width: f64) -> Route {
        Route {
            path: vec![],
            head: self.draw().start(from),
            width,
        }
    }

    fn route_finish(&mut self, route: Route, into: DotIndex) -> Result<(), ()> {
        self.draw().finish(route.head, into, route.width)?;
        Ok(())
    }

    fn route_path(&mut self, mut route: Route, path: &[DotIndex]) -> Result<Route, ()> {
        for dot in path {
            route = self.route_step(route, *dot)?;
        }

        Ok(route)
    }

    fn reroute_path(&mut self, mut route: Route, path: &[DotIndex]) -> Result<Route, ()> {
        let prefix_length = route
            .path
            .iter()
            .zip(path)
            .take_while(|(vertex, dot)| **vertex == self.mesh.vertex(**dot))
            .count();

        let length = route.path.len();
        route = self.unroute_steps(route, length - prefix_length)?;
        route = self.route_path(route, &path[prefix_length..])?;
        Ok(route)
    }

    fn unroute_step(&mut self, mut route: Route) -> Result<Route, ()> {
        route.head = self.draw().undo_segbend(route.head).unwrap();
        route.path.pop();
        Ok(route)
    }

    fn unroute_steps(&mut self, mut route: Route, step_count: usize) -> Result<Route, ()> {
        for _ in 0..step_count {
            route = self.unroute_step(route)?;
        }
        Ok(route)
    }

    fn route_step(&mut self, mut route: Route, to: DotIndex) -> Result<Route, ()> {
        route.head = self
            .draw()
            .segbend_around_dot(route.head, to, true, route.width)?;
        route.path.push(self.mesh.vertex(to));
        Ok(route)
    }

    pub fn squeeze_around_dot(
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
    }

    pub fn draw(&mut self) -> Draw {
        Draw::new(&mut self.layout, &self.rules)
    }

    pub fn routeedges(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.mesh.edge_references().map(|edge| {
            (
                self.mesh.position(edge.source()),
                self.mesh.position(edge.target()),
            )
        })
    }
}
