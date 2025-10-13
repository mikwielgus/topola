// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use geo::Point;

use crate::{
    board::{edit::BoardEdit, AccessMesadata},
    drawing::{
        band::BandTermsegIndex,
        dot::{FixedDotIndex, FixedDotWeight, GeneralDotWeight},
    },
    layout::LayoutEdit,
    math::Circle,
    router::{RouteStepper, Router},
    stepper::Step,
};

use super::{Autorouter, AutorouterError, PlanarAutorouteOptions};

pub struct PointrouteExecutionStepper {
    route: RouteStepper,
    options: PlanarAutorouteOptions,
}

impl PointrouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        origin: FixedDotIndex,
        point: Point,
        options: PlanarAutorouteOptions,
    ) -> Result<Self, AutorouterError> {
        let destination = autorouter.board.add_fixed_dot_infringably(
            &mut BoardEdit::new(), // TODO?
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: point,
                    r: options.router.routed_band_width / 2.0,
                },
                layer: 0,
                maybe_net: None,
            }),
            None,
        );

        let mut router = Router::new(autorouter.board.layout_mut(), options.router);

        Ok(Self {
            route: router.route(
                LayoutEdit::new(), // TODO?
                origin,
                destination,
                options.router.routed_band_width,
            )?,
            options,
        })
    }
}

impl<M: AccessMesadata> Step<Autorouter<M>, BandTermsegIndex> for PointrouteExecutionStepper {
    type Error = AutorouterError;

    fn step(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<ControlFlow<BandTermsegIndex>, AutorouterError> {
        let mut router = Router::new(autorouter.board.layout_mut(), self.options.router);
        Ok(self.route.step(&mut router)?)
    }
}
