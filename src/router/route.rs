use crate::{
    drawing::{
        band::BandTermsegIndex, dot::FixedDotIndex, graph::PrimitiveIndex, rules::AccessRules,
    },
    geometry::primitive::PrimitiveShape,
    router::{
        astar::{Astar, AstarError, AstarStatus},
        navcord::NavcordStepper,
        navcorder::Navcorder,
        navmesh::{Navmesh, NavmeshError},
        Router, RouterAstarStrategy, RouterStatus,
    },
    stepper::Step,
};

pub struct RouteStepper {
    astar: Astar<Navmesh, f64>,
    navcord: NavcordStepper,
    ghosts: Vec<PrimitiveShape>,
    obstacles: Vec<PrimitiveIndex>,
}

impl RouteStepper {
    pub fn new(
        router: &mut Router<impl AccessRules>,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<Self, NavmeshError> {
        let navmesh = Navmesh::new(router.layout(), from, to, router.options())?;
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

        let mut navcorder = Navcorder::new(router.layout_mut());
        let mut navcord = navcorder.start(source, source_navvertex, width);

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

    pub fn navcord(&self) -> &NavcordStepper {
        &self.navcord
    }

    pub fn ghosts(&self) -> &[PrimitiveShape] {
        &self.ghosts
    }

    pub fn obstacles(&self) -> &[PrimitiveIndex] {
        &self.obstacles
    }
}

impl<'a, R: AccessRules> Step<Router<'a, R>, RouterStatus, AstarError, BandTermsegIndex>
    for RouteStepper
{
    fn step(&mut self, router: &mut Router<R>) -> Result<RouterStatus, AstarError> {
        let navcorder = Navcorder::new(router.layout_mut());
        let target = self.astar.graph.destination();
        let mut strategy = RouterAstarStrategy::new(navcorder, &mut self.navcord, target);

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
