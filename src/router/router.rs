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
        dot::{DotIndex, FixedDotIndex},
        graph::{MakePrimitive, PrimitiveIndex},
        guide::Guide,
        head::GetFace,
        primitive::MakePrimitiveShape,
        rules::AccessRules,
        Collision, DrawingException, Infringement,
    },
    geometry::{
        primitive::PrimitiveShape,
        shape::{AccessShape, MeasureLength},
    },
    graph::{GetPetgraphIndex, MakeRef},
    layout::{Layout, LayoutEdit},
};

use super::{
    astar::{AstarStrategy, PathTracker},
    draw::DrawException,
    navcord::{NavcordStepContext, NavcordStepper},
    navcorder::{Navcorder, NavcorderException},
    navmesh::{Navmesh, NavmeshEdgeReference, NavmeshError, NavvertexIndex},
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
    pub navcord: &'a mut NavcordStepper,
    pub target: FixedDotIndex,
    pub probe_ghosts: Vec<PrimitiveShape>,
    pub probe_obstacles: Vec<PrimitiveIndex>,
}

impl<'a, R> RouterAstarStrategy<'a, R> {
    pub fn new(
        layout: &'a mut Layout<R>,
        navcord: &'a mut NavcordStepper,
        target: FixedDotIndex,
    ) -> Self {
        Self {
            layout,
            navcord,
            target,
            probe_ghosts: vec![],
            probe_obstacles: vec![],
        }
    }
}

impl<'a, R: AccessRules> RouterAstarStrategy<'a, R> {
    fn bihead_length(&self) -> f64 {
        self.navcord.head.ref_(self.layout.drawing()).length()
            + match self.navcord.head.face() {
                DotIndex::Fixed(..) => 0.0,
                DotIndex::Loose(face) => self
                    .layout
                    .drawing()
                    .rear_head(face)
                    .ref_(self.layout.drawing())
                    .length(),
            }
    }
}

impl<'a, R: AccessRules> AstarStrategy<Navmesh, f64, BandTermsegIndex>
    for RouterAstarStrategy<'a, R>
{
    fn is_goal(
        &mut self,
        navmesh: &Navmesh,
        vertex: NavvertexIndex,
        tracker: &PathTracker<Navmesh>,
    ) -> Option<BandTermsegIndex> {
        let new_path = tracker.reconstruct_path_to(vertex);
        let width = self.navcord.width;

        self.layout
            .rework_path(navmesh, self.navcord, &new_path[..], width)
            .unwrap();

        self.layout
            .finish(navmesh, self.navcord, self.target, width)
            .ok()
    }

    fn place_probe(&mut self, navmesh: &Navmesh, edge: NavmeshEdgeReference) -> Option<f64> {
        if edge.target().petgraph_index() == self.target.petgraph_index() {
            return None;
        }

        let prev_bihead_length = self.bihead_length();

        let width = self.navcord.width;
        let result = self.navcord.step(&mut NavcordStepContext {
            layout: self.layout,
            navmesh,
            to: edge.target(),
            width,
        });

        let probe_length = self.bihead_length() - prev_bihead_length;

        match result {
            Ok(..) => Some(probe_length),
            Err(err) => {
                if let NavcorderException::CannotDraw(draw_err) = err {
                    let layout_err = match draw_err {
                        DrawException::NoTangents(..) => return None,
                        DrawException::CannotFinishIn(.., layout_err) => layout_err,
                        DrawException::CannotWrapAround(.., layout_err) => layout_err,
                    };

                    let (ghost, obstacle) = match layout_err {
                        DrawingException::NoTangents(..) => return None,
                        DrawingException::Infringement(Infringement(ghost, obstacle)) => {
                            (ghost, obstacle)
                        }
                        DrawingException::Collision(Collision(ghost, obstacle)) => {
                            (ghost, obstacle)
                        }
                        DrawingException::AlreadyConnected(..) => return None,
                    };

                    self.probe_ghosts = vec![ghost];
                    self.probe_obstacles = vec![obstacle];
                }
                None
            }
        }
    }

    fn remove_probe(&mut self, _navmesh: &Navmesh) {
        self.navcord.step_back(&mut self.layout);
    }

    fn estimate_cost(&mut self, navmesh: &Navmesh, vertex: NavvertexIndex) -> f64 {
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
