// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use topola::{
    autorouter::{
        ratsnest::Ratsnest,
        selection::{BboxSelectionKind, Selection},
    },
    board::{AccessMesadata, Board},
    layout::NodeIndex,
    router::planar_incr_embed,
};

use crate::appearance_panel::AppearancePanel;

#[derive(Clone, Copy, Debug)]
pub struct PieNavmeshBase;

impl planar_incr_embed::NavmeshBase for PieNavmeshBase {
    type PrimalNodeIndex = NodeIndex;
    type EtchedPath = planar_incr_embed::navmesh::EdgeIndex<NodeIndex>;
    type GapComment = ();
    type Scalar = f64;
}

pub type PieNavmesh = planar_incr_embed::navmesh::Navmesh<PieNavmeshBase>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Addition,
    Substitution,
    Toggling,
}

pub struct Overlay {
    ratsnest: Ratsnest,
    pub selection: Selection,
    planar_incr_navmesh: Option<PieNavmesh>,
    reselect_bbox: Option<(SelectionMode, Point)>,
}

const INF: f64 = f64::INFINITY;

impl Overlay {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(board.layout())?,
            selection: Selection::new(),
            planar_incr_navmesh: None,
            reselect_bbox: None,
        })
    }

    pub fn take_selection(&mut self) -> Selection {
        core::mem::replace(&mut self.selection, Selection::new())
    }

    pub fn select_all(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
    ) {
        self.select_all_in_bbox(
            board,
            appearance_panel,
            &AABB::from_corners([-INF, -INF], [INF, INF]),
            BboxSelectionKind::CompletelyInside,
        );
    }

    pub fn unselect_all(&mut self) {
        self.selection = Selection::new();
        self.reselect_bbox = None;
    }

    pub fn recalculate_topo_navmesh(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
    ) {
        use spade::Triangulation;
        use topola::router::planar_incr_embed::navmesh::TrianVertex;

        let Some(active_layer) = appearance_panel.active_layer else {
            return;
        };

        if let Ok(triangulation) =
            spade::DelaunayTriangulation::<TrianVertex<NodeIndex, f64>>::bulk_load(
                board
                    .layout()
                    .drawing()
                    .rtree()
                    .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                        [-f64::INFINITY, -f64::INFINITY, active_layer as f64],
                        [f64::INFINITY, f64::INFINITY, active_layer as f64],
                    ))
                    .map(|&geom| geom.data)
                    .filter_map(|node| {
                        board
                            .layout()
                            .center_of_compoundless_node(node)
                            .map(|pos| (node, pos))
                    })
                    .map(|(idx, pos)| TrianVertex {
                        idx,
                        pos: spade::mitigate_underflow(spade::Point2 {
                            x: pos.x(),
                            y: pos.y(),
                        }),
                    })
                    .collect(),
            )
        {
            self.planar_incr_navmesh = Some(
                planar_incr_embed::navmesh::NavmeshSer::<PieNavmeshBase>::from_triangulation(
                    &triangulation,
                )
                .into(),
            );
        }
    }

    pub fn drag_start(
        &mut self,
        _board: &Board<impl AccessMesadata>,
        _appearance_panel: &AppearancePanel,
        at: Point,
        modifiers: &egui::Modifiers,
    ) {
        if self.reselect_bbox.is_none() {
            // handle bounding box selection
            let selmode = if modifiers.ctrl {
                SelectionMode::Toggling
            } else if modifiers.shift {
                SelectionMode::Addition
            } else {
                SelectionMode::Substitution
            };
            self.reselect_bbox = Some((selmode, at));
        }
    }

    pub fn drag_stop(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
        at: Point,
    ) {
        if let Some((selmode, bsk, aabb)) = self.get_bbox_reselect(at) {
            // handle bounding box selection
            self.reselect_bbox = None;

            match selmode {
                SelectionMode::Substitution => {
                    self.selection = Selection::new();
                    self.select_all_in_bbox(board, appearance_panel, &aabb, bsk);
                }
                SelectionMode::Addition => {
                    self.select_all_in_bbox(board, appearance_panel, &aabb, bsk);
                }
                SelectionMode::Toggling => {
                    let old_selection = self.take_selection();
                    self.select_all_in_bbox(board, appearance_panel, &aabb, bsk);
                    self.selection ^= &old_selection;
                }
            }
        }
    }

    pub fn click(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
        at: Point,
    ) {
        if self.reselect_bbox.is_some() {
            // handle bounding box selection (takes precendence over other interactions)
            // this is mostly in order to allow the user to recover from a missed/dropped drag_stop event
            self.drag_stop(board, appearance_panel, at);
            return;
        }

        let old_selection = self.take_selection();
        self.select_all_in_bbox(
            board,
            appearance_panel,
            &AABB::from_point([at.x(), at.y()]),
            BboxSelectionKind::MerelyIntersects,
        );
        self.selection ^= &old_selection;
    }

    pub fn select_all_in_bbox(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
        aabb: &AABB<[f64; 2]>,
        bsk: BboxSelectionKind,
    ) {
        self.selection
            .select_all_in_bbox(board, aabb, &appearance_panel.visible[..], bsk);
    }

    pub fn ratsnest(&self) -> &Ratsnest {
        &self.ratsnest
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn planar_incr_navmesh(&self) -> Option<&PieNavmesh> {
        self.planar_incr_navmesh.as_ref()
    }

    /// Returns the currently selected bounding box of a bounding-box reselect
    pub fn get_bbox_reselect(
        &self,
        at: Point,
    ) -> Option<(SelectionMode, BboxSelectionKind, AABB<[f64; 2]>)> {
        self.reselect_bbox.map(|(selmode, pt)| {
            (
                selmode,
                // Δx = at.x() - pt.x()
                if at.x() <= pt.x() {
                    // Δx ≤ 0
                    BboxSelectionKind::MerelyIntersects
                } else {
                    // Δx > 0
                    BboxSelectionKind::CompletelyInside
                },
                AABB::from_corners([pt.x(), pt.y()], [at.x(), at.y()]),
            )
        })
    }
}
