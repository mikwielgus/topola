// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;
use rstar::AABB;
use spade::InsertionError;

use topola::{
    autorouter::{
        selection::{BboxSelectionKind, Selection},
        Autorouter,
    },
    board::{AccessMesadata, Board},
    router::ng::{calculate_navmesh as ng_calculate_navmesh, PieNavmesh},
};

use crate::appearance_panel::AppearancePanel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Addition,
    Substitution,
    Toggling,
}

pub struct Overlay {
    pub selection: Selection,
    planar_incr_navmesh: Option<PieNavmesh>,
    reselect_bbox: Option<(SelectionMode, Point)>,
}

const INF: f64 = f64::INFINITY;

impl Overlay {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self {
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
        autorouter: &Autorouter<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
    ) {
        self.select_all_in_bbox(
            autorouter.board(),
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
        autorouter: &Autorouter<impl AccessMesadata>,
        active_layer: usize,
    ) {
        if let Ok(pien) = ng_calculate_navmesh(autorouter.board(), active_layer) {
            self.planar_incr_navmesh = Some(pien);
        }
    }

    pub fn drag_start(
        &mut self,
        _autorouter: &Autorouter<impl AccessMesadata>,
        _appearance_panel: &AppearancePanel,
        at: Point,
        ctrl: bool,
        shift: bool,
    ) {
        if self.reselect_bbox.is_none() {
            // handle bounding box selection
            let selmode = if ctrl {
                SelectionMode::Toggling
            } else if shift {
                SelectionMode::Addition
            } else {
                SelectionMode::Substitution
            };
            self.reselect_bbox = Some((selmode, at));
        }
    }

    pub fn drag_stop(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
        at: Point,
    ) {
        if let Some((selmode, bsk, aabb)) = self.get_bbox_reselect(at) {
            // handle bounding box selection
            self.reselect_bbox = None;

            match selmode {
                SelectionMode::Substitution => {
                    self.selection = Selection::new();
                    self.select_all_in_bbox(autorouter.board(), appearance_panel, &aabb, bsk);
                }
                SelectionMode::Addition => {
                    self.select_all_in_bbox(autorouter.board(), appearance_panel, &aabb, bsk);
                }
                SelectionMode::Toggling => {
                    let old_selection = self.take_selection();
                    self.select_all_in_bbox(autorouter.board(), appearance_panel, &aabb, bsk);
                    self.selection ^= &old_selection;
                }
            }
        }
    }

    pub fn click(
        &mut self,
        autorouter: &Autorouter<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
        at: Point,
    ) {
        if self.reselect_bbox.is_some() {
            // handle bounding box selection (takes precendence over other interactions)
            // this is mostly in order to allow the user to recover from a missed/dropped drag_stop event
            self.drag_stop(autorouter, appearance_panel, at);
            return;
        }

        let old_selection = self.take_selection();
        self.select_all_in_bbox(
            autorouter.board(),
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
