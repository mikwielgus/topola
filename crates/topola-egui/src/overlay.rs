// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use geo::Point;
use rstar::{Point as _, AABB};
use spade::InsertionError;

use topola::{
    autorouter::{
        ratsnest::Ratsnest,
        selection::{BboxSelectionKind, Selection},
    },
    board::{mesadata::AccessMesadata, Board},
    drawing::{
        graph::{GetLayer, MakePrimitive},
        primitive::MakePrimitiveShape,
    },
    geometry::shape::{AccessShape, Shape},
    graph::{GenericIndex, GetPetgraphIndex},
    layout::{
        poly::{MakePolygon, PolyWeight},
        via::ViaWeight,
        CompoundWeight, Layout, NodeIndex,
    },
};

use crate::appearance_panel::AppearancePanel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Addition,
    Substitution,
    Toggling,
}

pub struct Overlay {
    ratsnest: Ratsnest,
    selection: Selection,
    reselect_bbox: Option<(SelectionMode, Point)>,
}

const INF: f64 = f64::INFINITY;

impl Overlay {
    pub fn new(board: &Board<impl AccessMesadata>) -> Result<Self, InsertionError> {
        Ok(Self {
            ratsnest: Ratsnest::new(board.layout())?,
            selection: Selection::new(),
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

    pub fn drag_start(
        &mut self,
        board: &Board<impl AccessMesadata>,
        appearance_panel: &AppearancePanel,
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

        let geoms: Vec<_> = board
            .layout()
            .drawing()
            .rtree()
            .locate_in_envelope_intersecting(&AABB::<[f64; 3]>::from_corners(
                [at.x(), at.y(), -f64::INFINITY],
                [at.x(), at.y(), f64::INFINITY],
            ))
            .collect();

        if let Some(geom) = geoms.iter().find(|&&geom| {
            board.layout().node_shape(geom.data).contains_point(at)
                // TODO: fix which layers to query
                && board
                    .layout()
                    .drawing()
                    // This should use:
                    // `.is_node_in_any_layer_of(geom.data, &appearance_panel.visible[..])`
                    // instead, but that doesn't work reliably
                    .is_node_in_layer(geom.data, 0)
        }) {
            self.selection.toggle_at_node(board, geom.data);
        }
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
