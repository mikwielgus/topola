// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Manages the execution of routing commands within the autorouting system.

use std::{cmp::Ordering, ops::ControlFlow};

use contracts_try::debug_requires;
use derive_getters::{Dissolve, Getters};
use enum_dispatch::enum_dispatch;
use geo::geometry::LineString;
use thiserror::Error;

use crate::{
    board::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::{edit::ApplyGeometryEdit, primitive::PrimitiveShape},
    graph::GenericIndex,
    layout::poly::PolyWeight,
    router::{
        navcord::Navcord,
        navmesh::{Navmesh, NavnodeIndex},
        ng,
        thetastar::ThetastarStepper,
    },
    stepper::Step,
};

use super::{
    autoroute::AutorouteExecutionStepper,
    compare_detours::CompareDetoursExecutionStepper,
    execution::{Command, ExecutionStepper},
    history::{History, HistoryError},
    measure_length::MeasureLengthExecutionStepper,
    place_via::PlaceViaExecutionStepper,
    remove_bands::RemoveBandsExecutionStepper,
    Autorouter, AutorouterError,
};

/// Trait for getting the information to display on the debug overlay,
#[enum_dispatch]
pub trait GetDebugOverlayData {
    /// Get the Theta* stepper. Most importantly, this gives us the access to
    /// the navmesh.
    fn maybe_thetastar(&self) -> Option<&ThetastarStepper<Navmesh, f64>> {
        None
    }

    /// Obtain Topological/Planar Navigation Mesh, if present.
    fn maybe_topo_navmesh(&self) -> Option<ng::pie::navmesh::NavmeshRef<'_, ng::PieNavmeshBase>> {
        None
    }

    /// Get the navcord. This is useful for coloring the currently visited path
    /// on the navmesh.
    fn maybe_navcord(&self) -> Option<&Navcord> {
        None
    }

    fn active_polygons(&self) -> &[GenericIndex<PolyWeight>] {
        &[]
    }

    /// Get ghosts. Ghosts are the shapes that Topola attempted to create but
    /// failed due to them infringing on other shapes.
    fn ghosts(&self) -> &[PrimitiveShape] {
        &[]
    }

    /// Get the obstacles that prevented Topola from creating new objects (the
    /// shapes of these objects can be obtained from the above `.ghosts(...)`)
    /// method. This allows to highlight what prevented Topola's algorithm from
    /// going some way.
    fn obstacles(&self) -> &[PrimitiveIndex] {
        &[]
    }

    fn polygonal_blockers(&self) -> &[LineString] {
        &[]
    }

    /// Get a text string to show on the debug overlay near to a navnode.
    /// Usually returns None, this method exists only for quick and dirty
    /// debugging.
    fn navnode_debug_text(&self, _navnode: NavnodeIndex) -> Option<&str> {
        None
    }

    /// Get a text string to show on the debug overlay near to a navedge.
    /// Usually returns None, this method exists only for quick and dirty
    /// debugging.
    fn navedge_debug_text(&self, _navedge: (NavnodeIndex, NavnodeIndex)) -> Option<&str> {
        None
    }
}

/// Error types that can occur during the invocation of commands.
#[derive(Error, Debug, Clone)]
pub enum InvokerError {
    /// Wraps errors related to command history operations.
    #[error(transparent)]
    History(#[from] HistoryError),

    /// Wraps errors related to autorouter operations.
    #[error(transparent)]
    Autorouter(#[from] AutorouterError),
}

#[derive(Getters, Dissolve)]
pub struct Invoker<M> {
    /// Instance for executing desired autorouting commands.
    pub(super) autorouter: Autorouter<M>,
    /// History of executed commands.
    pub(super) history: History,
    /// Currently ongoing command type.
    pub(super) ongoing_command: Option<Command>,
}

impl<M: AccessMesadata + Clone> Invoker<M> {
    /// Creates a new instance of Invoker with the given autorouter instance
    pub fn new(autorouter: Autorouter<M>) -> Self {
        Self::new_with_history(autorouter, History::new())
    }

    /// Creates a new instance of Invoker with the given autorouter and history
    pub fn new_with_history(autorouter: Autorouter<M>, history: History) -> Self {
        Self {
            autorouter,
            history,
            ongoing_command: None,
        }
    }

    //#[debug_requires(self.ongoing_command.is_none())]
    /// Executes a command, managing the command status and history.
    ///
    /// This function is used to pass the [`Command`] to [`Invoker::execute_stepper`]
    /// function, and control its execution status.
    pub fn execute(&mut self, command: Command) -> Result<(), InvokerError> {
        let mut execute = self.execute_stepper(command)?;

        loop {
            let status = execute.step(self)?;

            if let ControlFlow::Break(..) = status {
                self.history.set_undone(std::iter::empty());
                return Ok(());
            }
        }
    }

    /// Pass given command to be executed.
    ///
    /// Function used to set given [`Command`] to ongoing state, dispatch and execute it.
    #[debug_requires(self.ongoing_command.is_none())]
    pub fn execute_stepper(
        &mut self,
        command: Command,
    ) -> Result<ExecutionStepper<M>, InvokerError> {
        let execute = self.dispatch_command(&command)?;
        self.ongoing_command = Some(command);
        Ok(execute)
    }

    #[debug_requires(self.ongoing_command.is_none())]
    fn dispatch_command(&mut self, command: &Command) -> Result<ExecutionStepper<M>, InvokerError> {
        Ok(match command {
            Command::Autoroute(selection, options) => {
                let mut ratlines = self.autorouter.selected_ratlines(selection);

                if options.presort_by_pairwise_detours {
                    ratlines.sort_unstable_by(|a, b| {
                        let mut compare_detours = self
                            .autorouter
                            .compare_detours_ratlines(*a, *b, *options)
                            .unwrap();
                        if let Ok((al, bl)) = compare_detours.finish(&mut self.autorouter) {
                            PartialOrd::partial_cmp(&al, &bl).unwrap()
                        } else {
                            Ordering::Equal
                        }
                    });
                }

                ExecutionStepper::Autoroute(self.autorouter.autoroute_ratlines(ratlines, *options)?)
            }
            Command::TopoAutoroute {
                selection,
                allowed_edges,
                active_layer,
                routed_band_width,
            } => {
                let ratlines = self.autorouter.selected_ratlines(selection);

                // TODO: consider "presort by pairwise detours"

                ExecutionStepper::TopoAutoroute(
                    self.autorouter.topo_autoroute_ratlines(
                        ratlines,
                        allowed_edges.clone(),
                        self.autorouter
                            .board
                            .layout()
                            .rules()
                            .layername_layer(active_layer)
                            .unwrap(),
                        *routed_band_width,
                        None,
                    )?,
                )
            }
            Command::PlaceVia(weight) => {
                ExecutionStepper::PlaceVia(self.autorouter.place_via(*weight)?)
            }
            Command::RemoveBands(selection) => {
                ExecutionStepper::RemoveBands(self.autorouter.remove_bands(selection)?)
            }
            Command::CompareDetours(selection, options) => ExecutionStepper::CompareDetours(
                self.autorouter.compare_detours(selection, *options)?,
            ),
            Command::MeasureLength(selection) => {
                ExecutionStepper::MeasureLength(self.autorouter.measure_length(selection)?)
            }
        })
    }

    #[debug_requires(self.ongoing_command.is_none())]
    /// Undo last command.
    pub fn undo(&mut self) -> Result<(), InvokerError> {
        let last_done = self.history.last_done()?;

        if let Some(edit) = last_done.edit() {
            self.autorouter.board.apply(&edit.reverse());
        }

        Ok(self.history.undo()?)
    }

    //#[debug_requires(self.ongoing_command.is_none())]
    /// Redo last command.
    pub fn redo(&mut self) -> Result<(), InvokerError> {
        let last_undone = self.history.last_undone()?;

        if let Some(edit) = last_undone.edit() {
            self.autorouter.board.apply(edit);
        }

        Ok(self.history.redo()?)
    }

    #[debug_requires(self.ongoing_command.is_none())]
    /// Replay last command.
    pub fn replay(&mut self, history: History) {
        let (done, undone) = history.dissolve();

        for entry in done {
            self.execute(entry.command().clone());
        }

        self.history.set_undone(undone);
    }
}
