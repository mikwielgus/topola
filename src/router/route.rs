use crate::{
    drawing::{dot::FixedDotIndex, graph::PrimitiveIndex, rules::AccessRules},
    geometry::primitive::PrimitiveShape,
    router::{
        astar::{Astar, AstarStatus},
        navmesh::Navmesh,
        trace::Trace,
        tracer::Tracer,
        Router, RouterAstarStrategy, RouterError, RouterStatus,
    },
    step::Step,
};

pub struct Route {
    astar: Astar<Navmesh, f64>,
    trace: Trace,
    ghosts: Vec<PrimitiveShape>,
    obstacles: Vec<PrimitiveIndex>,
}

impl Route {
    pub fn new(
        router: &mut Router<impl AccessRules>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Self, RouterError> {
        let navmesh = Navmesh::new(router.layout(), from, to)?;
        Ok(Self::new_from_navmesh(router, navmesh, width))
    }

    pub fn new_from_navmesh(
        router: &mut Router<impl AccessRules>,
        navmesh: Navmesh,
        width: f64,
    ) -> Self {
        let source = navmesh.origin();
        let source_navvertex = navmesh.origin_navvertex();
        let target = navmesh.destination();

        let mut tracer = Tracer::new(router.layout_mut());
        let mut trace = tracer.start(source, source_navvertex, width);

        let mut strategy = RouterAstarStrategy::new(tracer, &mut trace, target);
        let astar = Astar::new(navmesh, source_navvertex, &mut strategy);
        let ghosts = vec![];
        let obstacles = vec![];

        Self {
            astar,
            trace,
            ghosts,
            obstacles,
        }
    }

    pub fn navmesh(&self) -> &Navmesh {
        &self.astar.graph
    }

    pub fn trace(&self) -> &Trace {
        &self.trace
    }

    pub fn ghosts(&self) -> &[PrimitiveShape] {
        &self.ghosts
    }

    pub fn obstacles(&self) -> &[PrimitiveIndex] {
        &self.obstacles
    }
}

impl<'a, R: AccessRules> Step<Router<'a, R>, RouterStatus, RouterError, ()> for Route {
    fn step(&mut self, router: &mut Router<R>) -> Result<RouterStatus, RouterError> {
        let tracer = Tracer::new(router.layout_mut());
        let target = self.astar.graph.destination();
        let mut strategy = RouterAstarStrategy::new(tracer, &mut self.trace, target);

        let result = match self.astar.step(&mut strategy)? {
            AstarStatus::Probing | AstarStatus::Probed | AstarStatus::Visited => {
                Ok(RouterStatus::Running)
            }
            AstarStatus::Finished(_cost, _path, band) => Ok(RouterStatus::Finished(band)),
        };

        self.ghosts = strategy.probe_ghosts;
        self.obstacles = strategy.probe_obstacles;
        result
    }
}
