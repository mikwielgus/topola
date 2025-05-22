// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use derive_getters::{Dissolve, Getters};

use crate::{
    drawing::{
        band::BandTermsegIndex, dot::FixedDotIndex, graph::PrimitiveIndex, rules::AccessRules,
    },
    geometry::primitive::PrimitiveShape,
    layout::LayoutEdit,
    router::{
        astar::{AstarError, AstarStepper},
        navcord::Navcord,
        navcorder::Navcorder,
        navmesh::{Navmesh, NavmeshError},
        Router, RouterAstarStrategy,
    },
    stepper::Step,
};

#[derive(Getters, Dissolve)]
pub struct RouteStepper {
    astar: AstarStepper<Navmesh, f64>,
    navcord: Navcord,
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
        let navmesh = Navmesh::new(router.layout(), from, to, *router.options())?;
        Ok(Self::new_from_navmesh(router, recorder, navmesh, width))
    }

    pub fn new_from_navmesh(
        router: &mut Router<impl AccessRules>,
        recorder: LayoutEdit,
        navmesh: Navmesh,
        width: f64,
    ) -> Self {
        let source = navmesh.origin();
        let source_navnode = navmesh.origin_navnode();
        let target = navmesh.destination();

        let layout = router.layout_mut();
        let mut navcord = layout.start(recorder, source, source_navnode, width);

        let mut strategy = RouterAstarStrategy::new(layout, &mut navcord, target);
        let astar = AstarStepper::new(navmesh, source_navnode, &mut strategy);
        let ghosts = vec![];
        let obstacles = vec![];

        Self {
            astar,
            navcord,
            ghosts,
            obstacles,
        }
    }
}

impl<R: AccessRules> Step<Router<'_, R>, BandTermsegIndex> for RouteStepper {
    type Error = AstarError;

    fn step(
        &mut self,
        router: &mut Router<R>,
    ) -> Result<ControlFlow<BandTermsegIndex>, AstarError> {
        let layout = router.layout_mut();
        let target = self.astar.graph.destination();
        let mut strategy = RouterAstarStrategy::new(layout, &mut self.navcord, target);
        let result = self.astar.step(&mut strategy);
        self.ghosts = strategy.probe_ghosts;
        self.obstacles = strategy.probe_obstacles;

        match result {
            Ok(ControlFlow::Continue(..)) => Ok(ControlFlow::Continue(())),
            Ok(ControlFlow::Break((_cost, _path, band))) => Ok(ControlFlow::Break(band)),
            Err(e) => {
                // NOTE(fogti): The 1 instead 0 is because the first element in the path
                // is the source navnode. See also: `NavcordStepper::new`.
                for _ in 1..self.navcord.path.len() {
                    self.navcord.step_back(layout);
                }
                Err(e)
            }
        }
    }
}
