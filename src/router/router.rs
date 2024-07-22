use geo::EuclideanDistance;
use petgraph::{data::DataMap, visit::EdgeRef};
use thiserror::Error;

use crate::{
    drawing::{
        band::BandTerminatingSegIndex,
        dot::{DotIndex, FixedDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        head::GetFace,
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        Collision, Infringement, LayoutException,
    },
    geometry::{
        primitive::{AccessPrimitiveShape, PrimitiveShape},
        shape::{AccessShape, MeasureLength},
    },
    graph::{GetPetgraphIndex, MakeRef},
    layout::Layout,
};

use super::{
    astar::{AstarError, AstarStrategy, PathTracker},
    draw::DrawException,
    navmesh::{Navmesh, NavmeshEdgeReference, NavmeshError, NavvertexIndex},
    route::Route,
    trace::Trace,
    tracer::{Tracer, TracerException},
};

#[derive(Error, Debug, Clone)]
#[error("routing failed")]
pub enum RouterError {
    Navmesh(#[from] NavmeshError),
    Astar(#[from] AstarError),
}

#[derive(Debug)]
pub enum RouterStatus {
    Running,
    Finished(BandTerminatingSegIndex),
}

#[derive(Debug)]
pub struct RouterAstarStrategy<'a, R: AccessRules> {
    pub tracer: Tracer<'a, R>,
    pub trace: &'a mut Trace,
    pub target: FixedDotIndex,
    pub probe_ghosts: Vec<PrimitiveShape>,
    pub probe_obstacles: Vec<PrimitiveIndex>,
}

impl<'a, R: AccessRules> RouterAstarStrategy<'a, R> {
    pub fn new(tracer: Tracer<'a, R>, trace: &'a mut Trace, target: FixedDotIndex) -> Self {
        Self {
            tracer,
            trace,
            target,
            probe_ghosts: vec![],
            probe_obstacles: vec![],
        }
    }

    fn bihead_length(&self) -> f64 {
        self.trace.head.ref_(self.tracer.layout.drawing()).length()
            + match self.trace.head.face() {
                DotIndex::Fixed(..) => 0.0,
                DotIndex::Loose(face) => self
                    .tracer
                    .layout
                    .drawing()
                    .guide()
                    .rear_head(face)
                    .ref_(self.tracer.layout.drawing())
                    .length(),
            }
    }
}

impl<'a, R: AccessRules> AstarStrategy<Navmesh, f64, BandTerminatingSegIndex>
    for RouterAstarStrategy<'a, R>
{
    fn is_goal(
        &mut self,
        navmesh: &Navmesh,
        vertex: NavvertexIndex,
        tracker: &PathTracker<Navmesh>,
    ) -> Option<BandTerminatingSegIndex> {
        let new_path = tracker.reconstruct_path_to(vertex);
        let width = self.trace.width;

        self.tracer
            .rework_path(navmesh, &mut self.trace, &new_path[..], width)
            .unwrap();

        self.tracer
            .finish(navmesh, &mut self.trace, self.target, width)
            .ok()
    }

    fn place_probe(&mut self, navmesh: &Navmesh, edge: NavmeshEdgeReference) -> Option<f64> {
        if edge.target().petgraph_index() == self.target.petgraph_index() {
            return None;
        }

        let prev_bihead_length = self.bihead_length();

        let width = self.trace.width;
        let result = self
            .trace
            .step(&mut self.tracer, navmesh, edge.target(), width);

        let probe_length = self.bihead_length() - prev_bihead_length;

        match result {
            Ok(..) => Some(probe_length),
            Err(err) => {
                if let TracerException::CannotDraw(draw_err) = err {
                    let layout_err = match draw_err {
                        DrawException::NoTangents(..) => return None,
                        DrawException::CannotFinishIn(.., layout_err) => layout_err,
                        DrawException::CannotWrapAround(.., layout_err) => layout_err,
                    };

                    let (ghost, obstacle) = match layout_err {
                        LayoutException::NoTangents(..) => return None,
                        LayoutException::Infringement(Infringement(ghost, obstacle)) => {
                            (ghost, obstacle)
                        }
                        LayoutException::Collision(Collision(ghost, obstacle)) => (ghost, obstacle),
                        LayoutException::AlreadyConnected(..) => return None,
                    };

                    self.probe_ghosts = vec![ghost];
                    self.probe_obstacles = vec![obstacle];
                }
                None
            }
        }
    }

    fn remove_probe(&mut self, _navmesh: &Navmesh) {
        self.trace.undo_step(&mut self.tracer);
    }

    fn estimate_cost(&mut self, navmesh: &Navmesh, vertex: NavvertexIndex) -> f64 {
        let start_point = PrimitiveIndex::from(navmesh.node_weight(vertex).unwrap().node)
            .primitive(self.tracer.layout.drawing())
            .shape()
            .center();
        let end_point = self
            .tracer
            .layout
            .drawing()
            .primitive(self.target)
            .shape()
            .center();

        end_point.euclidean_distance(&start_point)
    }
}

#[derive(Debug)]
pub struct Router<'a, R: AccessRules> {
    layout: &'a mut Layout<R>,
}

impl<'a, R: AccessRules> Router<'a, R> {
    pub fn new(layout: &'a mut Layout<R>) -> Self {
        Self { layout }
    }

    pub fn route(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<BandTerminatingSegIndex, RouterError> {
        let mut route = self.route_walk(from, to, width)?;

        loop {
            let status = match route.step(self) {
                Ok(status) => status,
                Err(err) => return Err(err),
            };

            if let RouterStatus::Finished(band) = status {
                return Ok(band);
            }
        }
    }

    pub fn route_walk(
        &mut self,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Route, RouterError> {
        Route::new(self, from, to, width)
    }

    pub fn layout_mut(&mut self) -> &mut Layout<R> {
        &mut self.layout
    }

    pub fn layout(&self) -> &Layout<R> {
        &self.layout
    }
}
