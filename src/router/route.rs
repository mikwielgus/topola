use std::ops::ControlFlow;

use derive_getters::{Dissolve, Getters};

use crate::{
    drawing::{
        band::BandTermsegIndex, dot::FixedDotIndex, graph::PrimitiveIndex, rules::AccessRules,
    },
    geometry::primitive::PrimitiveShape,
    layout::LayoutEdit,
    router::{
        astar::{Astar, AstarError},
        navcord::NavcordStepper,
        navcorder::Navcorder,
        navmesh::{Navmesh, NavmeshError},
        Router, RouterAstarStrategy,
    },
    stepper::Step,
};

#[derive(Getters, Dissolve)]
pub struct RouteStepper {
    #[getter(skip)]
    astar: Astar<Navmesh, f64>,
    navcord: NavcordStepper,
    ghosts: Vec<PrimitiveShape>,
    obstacles: Vec<PrimitiveIndex>,
}

impl RouteStepper {
    pub fn new(
        router: &mut Router<impl AccessRules>,
        recorder: LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Self, NavmeshError> {
        let navmesh = Navmesh::new(router.layout(), from, to, router.options().clone())?;
        Ok(Self::new_from_navmesh(router, recorder, navmesh, width))
    }

    pub fn new_from_navmesh(
        router: &mut Router<impl AccessRules>,
        recorder: LayoutEdit,
        navmesh: Navmesh,
        width: f64,
    ) -> Self {
        let source = navmesh.origin();
        let source_navvertex = navmesh.origin_navvertex();
        let target = navmesh.destination();

        let mut navcorder = Navcorder::new(router.layout_mut());
        let mut navcord = navcorder.start(recorder, source, source_navvertex, width);

        let mut strategy = RouterAstarStrategy::new(navcorder, &mut navcord, target);
        let astar = Astar::new(navmesh, source_navvertex, &mut strategy);
        let ghosts = vec![];
        let obstacles = vec![];

        Self {
            astar,
            navcord,
            ghosts,
            obstacles,
        }
    }

    pub fn navmesh(&self) -> &Navmesh {
        &self.astar.graph
    }
}

impl<'a, R: AccessRules> Step<Router<'a, R>, BandTermsegIndex> for RouteStepper {
    type Error = AstarError;

    fn step(
        &mut self,
        router: &mut Router<R>,
    ) -> Result<ControlFlow<BandTermsegIndex>, AstarError> {
        let navcorder = Navcorder::new(router.layout_mut());
        let target = self.astar.graph.destination();
        let mut strategy = RouterAstarStrategy::new(navcorder, &mut self.navcord, target);

        let result = match self.astar.step(&mut strategy)? {
            ControlFlow::Continue(..) => Ok(ControlFlow::Continue(())),
            ControlFlow::Break((_cost, _path, band)) => Ok(ControlFlow::Break(band)),
        };

        self.ghosts = strategy.probe_ghosts;
        self.obstacles = strategy.probe_obstacles;
        result
    }
}
