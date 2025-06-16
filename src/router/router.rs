// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::Getters;
use geo::algorithm::line_measures::{Distance, Euclidean};
use petgraph::data::DataMap;
use serde::{Deserialize, Serialize};

use crate::{
    drawing::{
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        primitive::MakePrimitiveShape,
        rules::AccessRules,
    },
    geometry::{primitive::PrimitiveShape, shape::AccessShape},
    layout::{Layout, LayoutEdit},
};

use super::{
    draw::DrawException,
    navcord::Navcord,
    navcorder::{Navcorder, NavcorderException},
    navmesh::{Navmesh, NavmeshError, NavnodeIndex},
    route::RouteStepper,
    thetastar::{PathTracker, ThetastarStrategy},
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RouterOptions {
    pub routed_band_width: f64,
    pub wrap_around_bands: bool,
    pub squeeze_through_under_bends: bool,
}

#[derive(Debug)]
pub struct RouterThetastarStrategy<'a, R> {
    pub layout: &'a mut Layout<R>,
    pub navcord: &'a mut Navcord,
    pub target: FixedDotIndex,
    pub probe_ghosts: Vec<PrimitiveShape>,
    pub probe_obstacles: Vec<PrimitiveIndex>,
}

impl<'a, R> RouterThetastarStrategy<'a, R> {
    pub fn new(layout: &'a mut Layout<R>, navcord: &'a mut Navcord, target: FixedDotIndex) -> Self {
        Self {
            layout,
            navcord,
            target,
            probe_ghosts: vec![],
            probe_obstacles: vec![],
        }
    }
}

impl<R: AccessRules> ThetastarStrategy<Navmesh, f64, BandTermsegIndex>
    for RouterThetastarStrategy<'_, R>
{
    fn visit_navnode(
        &mut self,
        navmesh: &Navmesh,
        navnode: NavnodeIndex,
        tracker: &PathTracker<Navmesh>,
    ) -> Result<Option<BandTermsegIndex>, ()> {
        let new_path = tracker.reconstruct_path_to(navnode);

        if navnode == navmesh.destination_navnode() {
            self.layout
                .rework_path(navmesh, self.navcord, &new_path[..new_path.len() - 1])
                .map_err(|_| ())?;

            // Set navcord members for consistency. The code would probably work
            // without this, since A* will terminate now anyway.
            self.navcord.maybe_final_termseg = Some(
                self.layout
                    .finish(navmesh, self.navcord, self.target)
                    .map_err(|_| ())?,
            );
            self.navcord.path.push(navnode);

            Ok(self.navcord.maybe_final_termseg)
        } else {
            self.layout
                .rework_path(navmesh, self.navcord, &new_path[..])
                .map_or(Err(()), |_| Ok(None))
        }
    }

    fn place_probe_to_navnode(
        &mut self,
        navmesh: &Navmesh,
        probed_navnode: NavnodeIndex,
    ) -> Option<f64> {
        let result = self.navcord.step_to(self.layout, navmesh, probed_navnode);

        match result {
            Ok(probe_length) => Some(probe_length),
            Err(err) => {
                if let NavcorderException::CannotDraw(draw_err) = err {
                    let layout_err = match draw_err {
                        DrawException::NoTangents(..) => return None,
                        DrawException::CannotFinishIn(.., layout_err) => layout_err,
                        DrawException::CannotWrapAround(.., layout_err) => layout_err,
                    };

                    let (ghost, obstacle) = layout_err.maybe_ghost_and_obstacle()?;
                    self.probe_ghosts = vec![*ghost];
                    self.probe_obstacles = vec![obstacle];
                }
                None
            }
        }
    }

    fn remove_probe(&mut self, _navmesh: &Navmesh) {
        self.navcord.step_back(self.layout);
    }

    fn estimate_cost(&mut self, navmesh: &Navmesh, vertex: NavnodeIndex) -> f64 {
        let start_point = PrimitiveIndex::from(navmesh.node_weight(vertex).unwrap().node)
            .primitive(self.layout.drawing())
            .shape()
            .center();
        let end_point = self
            .layout
            .drawing()
            .primitive(self.target)
            .shape()
            .center();

        Euclidean::distance(&end_point, &start_point)
    }
}

#[derive(Debug, Getters)]
pub struct Router<'a, R: AccessRules> {
    #[getter(skip)]
    layout: &'a mut Layout<R>,
    options: RouterOptions,
}

impl<'a, R: AccessRules> Router<'a, R> {
    pub fn new(layout: &'a mut Layout<R>, options: RouterOptions) -> Self {
        Self { layout, options }
    }

    pub fn route(
        &mut self,
        recorder: LayoutEdit,
        from: FixedDotIndex,
        to: FixedDotIndex,
        width: f64,
    ) -> Result<RouteStepper, NavmeshError> {
        RouteStepper::new(self, recorder, from, to, width)
    }

    pub fn layout_mut(&mut self) -> &mut Layout<R> {
        self.layout
    }

    pub fn layout(&self) -> &Layout<R> {
        self.layout
    }
}
