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
        navcord::Navcord,
        navcorder::Navcorder,
        navmesh::{Navmesh, NavmeshError},
        thetastar::{ThetastarError, ThetastarStepper},
        Router, RouterThetastarStrategy,
    },
    stepper::{EstimateProgress, LinearScale, Step},
};

#[derive(Getters, Dissolve)]
pub struct RouteStepper {
    thetastar: ThetastarStepper<Navmesh, f64>,
    navcord: Navcord,
    ghosts: Vec<PrimitiveShape>,
    obstacles: Vec<PrimitiveIndex>,
}

impl RouteStepper {
    pub fn new(
        router: &mut Router<impl AccessRules>,
        recorder: LayoutEdit,
        origin: FixedDotIndex,
        destination: FixedDotIndex,
        width: f64,
    ) -> Result<Self, NavmeshError> {
        let navmesh = Navmesh::new(router.layout(), origin, destination, *router.options())?;
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

        let mut strategy = RouterThetastarStrategy::new(layout, &mut navcord, target);
        let thetastar = ThetastarStepper::new(navmesh, source_navnode, &mut strategy);
        let ghosts = vec![];
        let obstacles = vec![];

        Self {
            thetastar,
            navcord,
            ghosts,
            obstacles,
        }
    }
}

impl<R: AccessRules> Step<Router<'_, R>, BandTermsegIndex> for RouteStepper {
    type Error = ThetastarError;

    fn step(
        &mut self,
        router: &mut Router<R>,
    ) -> Result<ControlFlow<BandTermsegIndex>, ThetastarError> {
        let layout = router.layout_mut();
        let target = self.thetastar.graph().destination();
        let mut strategy = RouterThetastarStrategy::new(layout, &mut self.navcord, target);
        let result = self.thetastar.step(&mut strategy);

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

impl EstimateProgress for RouteStepper {
    type Value = f64;
    type Subscale = ();

    fn estimate_progress(&self) -> LinearScale<f64> {
        self.thetastar.estimate_progress()
    }
}
