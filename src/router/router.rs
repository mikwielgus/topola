// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use derive_getters::Getters;
use geo::algorithm::line_measures::{Distance, Euclidean};
use petgraph::{data::DataMap, visit::EdgeRef};
use serde::{Deserialize, Serialize};

use crate::{
    drawing::{
        band::BandTermsegIndex,
        dot::FixedDotIndex,
        graph::{MakePrimitive, PrimitiveIndex},
        head::Head,
        primitive::MakePrimitiveShape,
        rules::AccessRules,
    },
    geometry::{
        primitive::PrimitiveShape,
        shape::{AccessShape, MeasureLength},
    },
    graph::MakeRef,
    layout::{Layout, LayoutEdit},
};

use super::{
    astar::{AstarStrategy, PathTracker},
    draw::DrawException,
    navcord::Navcord,
    navcorder::{Navcorder, NavcorderException},
    navmesh::{Navmesh, NavmeshEdgeReference, NavmeshError, NavnodeIndex},
    route::RouteStepper,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RouterOptions {
    pub routed_band_width: f64,
    pub wrap_around_bands: bool,
    pub squeeze_through_under_bends: bool,
}

#[derive(Debug)]
pub struct RouterAstarStrategy<'a, R> {
    pub layout: &'a mut Layout<R>,
    pub navcord: &'a mut Navcord,
    pub target: FixedDotIndex,
    pub probe_ghosts: Vec<PrimitiveShape>,
    pub probe_obstacles: Vec<PrimitiveIndex>,
}

impl<'a, R> RouterAstarStrategy<'a, R> {
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

impl<R: AccessRules> AstarStrategy<Navmesh, f64, BandTermsegIndex> for RouterAstarStrategy<'_, R> {
    fn visit_navnode(
        &mut self,
        navmesh: &Navmesh,
        vertex: NavnodeIndex,
        tracker: &PathTracker<Navmesh>,
    ) -> Result<Option<BandTermsegIndex>, ()> {
        let new_path = tracker.reconstruct_path_to(vertex);

        if vertex == navmesh.destination_navnode() {
            self.layout
                .rework_path(navmesh, self.navcord, &new_path[..new_path.len() - 1])
                .unwrap();

            // Set navcord members for consistency. The code would probably work
            // without this, since A* will terminate now anyway.
            self.navcord.final_termseg = Some(
                self.layout
                    .finish(navmesh, self.navcord, self.target)
                    .unwrap(),
            );
            self.navcord.path.push(vertex);

            Ok(self.navcord.final_termseg)
        } else {
            self.layout
                .rework_path(navmesh, self.navcord, &new_path[..])
                .map_or(Err(()), |_| Ok(None))
        }
    }

    fn place_probe_at_navedge(
        &mut self,
        navmesh: &Navmesh,
        edge: NavmeshEdgeReference,
    ) -> Option<f64> {
        let old_head = self.navcord.head;
        let result = self.navcord.step_to(self.layout, navmesh, edge.target());

        let prev_bend_length = match old_head {
            Head::Cane(old_cane_head) => self
                .layout
                .drawing()
                .primitive(old_cane_head.cane.bend)
                .shape()
                .length(),
            Head::Bare(..) => 0.0,
        };

        let probe_length = prev_bend_length
            // NOTE: the probe's bend length is always 0 here because such is
            // the initial state of a cane (before getting extended, but this
            // is never done for probes). So we could as well only measure the
            // seg's length.
            + self.navcord.head.ref_(self.layout.drawing()).length();

        match result {
            Ok(..) => Some(probe_length),
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
