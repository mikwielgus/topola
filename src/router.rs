use geo::geometry::Point;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use spade::InsertionError;



use crate::astar::astar;
use crate::bow::Bow;
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
            |node, _tracker| (node != self.mesh.vertex(to)).then_some(0),
            |_edge| 1,
            |_| 0,
        )
        .unwrap(); // TODO.

        let path: Vec<DotIndex> = mesh_path
            .iter()
            .map(|vertex| self.mesh.dot(*vertex))
            .collect();

        self.route_path(&path[..], 5.0).unwrap(); // TODO.

        Ok(())
    }

    fn route_path(&mut self, path: &[DotIndex], width: f64) -> Result<(), ()> {
        let mut route = self.route_start(path[0], width);

        for dot in &path[1..(path.len() - 1)] {
            route = self.route_step(route, *dot)?;
        }

        self.route_finish(route, path[path.len() - 1])
    }

    fn route_start(&mut self, from: DotIndex, width: f64) -> Route {
        Route {
            path: vec![],
            head: self.draw_start(from),
            width,
        }
    }

    fn route_finish(&mut self, route: Route, into: DotIndex) -> Result<(), ()> {
        self.draw_finish(route.head, into, route.width)?;
        Ok(())
    }

    fn route_step(&mut self, mut route: Route, to: DotIndex) -> Result<Route, ()> {
        route.head = self.draw_around_dot(route.head, to, true, route.width)?;
        route.path.push(self.mesh.vertex(to));
        Ok(route)
    }

    pub fn draw_start(&mut self, from: DotIndex) -> Head {
        Head {
            dot: from,
            segbend: self.layout.prev_segbend(from),
        }
    }

    pub fn draw_finish(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        if let Some(bend) = self.layout.primitive(into).bend() {
            self.draw_finish_in_bend(head, bend, into, width)?;
        } else {
            self.draw_finish_in_dot(head, into, width)?;
        }

        Ok(())
    }

    fn draw_finish_in_dot(&mut self, head: Head, into: DotIndex, width: f64) -> Result<(), ()> {
        let tangent = self
            .guide(&Default::default())
            .head_into_dot_segment(&head, into, width);
        let head = self.extend_head(head, tangent.start_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        Ok(())
    }

    fn draw_finish_in_bend(
        &mut self,
        head: Head,
        into_bend: BendIndex,
        into: DotIndex,
        width: f64,
    ) -> Result<(), ()> {
        let to_head = Head {
            dot: into,
            segbend: self.layout.next_segbend(into),
        };
        let to_cw = self.guide(&Default::default()).head_cw(&to_head).unwrap();
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, into_bend, to_cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        let _to_head = self.extend_head(to_head, tangent.end_point())?;

        let net = self.layout.primitive(head.dot).weight().net;
        self.layout
            .add_seg(head.dot, into, SegWeight { net, width })?;
        Ok(())
    }

    pub fn squeeze_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw_around_dot(head, around, cw, width)?;
        self.layout
            .reattach_bend(outer, head.segbend.as_ref().unwrap().bend);

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn draw_around_dot(
        &mut self,
        head: Head,
        around: DotIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_dot_segment(&head, around, cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        self.draw_segbend(
            head,
            TaggedIndex::Dot(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    pub fn squeeze_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let outer = self.layout.primitive(around).outer().unwrap();
        let head = self.draw_around_bend(head, around, cw, width)?;
        self.layout
            .reattach_bend(outer, head.segbend.as_ref().unwrap().bend);

        self.reroute_outward(outer)?;
        Ok(head)
    }

    pub fn draw_around_bend(
        &mut self,
        head: Head,
        around: BendIndex,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let tangent = self
            .guide(&Default::default())
            .head_around_bend_segment(&head, around, cw, width);

        let head = self.extend_head(head, tangent.start_point())?;
        self.draw_segbend(
            head,
            TaggedIndex::Bend(around),
            tangent.end_point(),
            cw,
            width,
        )
    }

    fn draw_segbend(
        &mut self,
        head: Head,
        around: TaggedIndex,
        to: Point,
        cw: bool,
        width: f64,
    ) -> Result<Head, ()> {
        let (head, seg) = self.draw_seg(head, to, width)?;
        let dot = head.dot;
        let bend_to = self
            .layout
            .add_dot(self.layout.primitive(head.dot).weight())?;
        let net = self.layout.primitive(head.dot).weight().net;

        let bend = self
            .layout
            .add_bend(head.dot, bend_to, around, BendWeight { net, cw })?;
        Ok(Head {
            dot: bend_to,
            segbend: Some(Segbend { bend, dot, seg }),
        })
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
            let mut head = self.draw_start(ends.0);
            let width = 5.0;

            if let Some(inner) = maybe_inner {
                head = self.draw_around_bend(head, inner, cw, width)?;
            } else {
                head = self.draw_around_dot(head, core, cw, width)?;
            }

            maybe_inner = head.segbend.as_ref().map(|segbend| segbend.bend);
            self.draw_finish(head, ends.1, width)?;
            self.relax_band(maybe_inner.unwrap());
        }

        Ok(())
    }

    fn draw_seg(&mut self, head: Head, to: Point, width: f64) -> Result<(Head, SegIndex), ()> {
        let net = self.layout.primitive(head.dot).weight().net;

        assert!(width <= self.layout.primitive(head.dot).weight().circle.r * 2.);

        let to_index = self.layout.add_dot(DotWeight {
            net,
            circle: Circle {
                pos: to,
                r: width / 2.0,
            },
        })?;
        let seg = self
            .layout
            .add_seg(head.dot, to_index, SegWeight { net, width })?;
        Ok((
            Head {
                dot: to_index,
                segbend: None,
            },
            seg,
        ))
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

        let head = self.draw_start(ends.0);
        let _ = self.draw_finish(head, ends.1, 5.0);
    }

    pub fn move_dot(&mut self, dot: DotIndex, to: Point) -> Result<(), ()> {
        self.layout.move_dot(dot, to)?;

        if let Some(outer) = self.layout.primitive(dot).outer() {
            self.reroute_outward(outer)?;
        }

        Ok(())
    }

    fn extend_head(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        if let Some(..) = head.segbend {
            self.extend_head_bend(head, to)
        } else {
            Ok(head)
            // No assertion for now because we temporarily use floats.

            //println!("{:?} {:?}", self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos, to);
            //assert!(self.layout.weight(TaggedIndex::Dot(from)).as_dot().unwrap().circle.pos == to);
        }
    }

    fn extend_head_bend(&mut self, head: Head, to: Point) -> Result<Head, ()> {
        self.layout
            .extend_bend(head.segbend.as_ref().unwrap().bend, head.dot, to)?;
        Ok(head)
    }

    fn guide<'a>(&'a self, conditions: &'a Conditions) -> Guide {
        Guide::new(&self.layout, &self.rules, conditions)
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
