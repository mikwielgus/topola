// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

use std::ops::ControlFlow;

use geo::Point;

use crate::{
    board::AccessMesadata,
    drawing::{
        band::BandTermsegIndex,
        dot::{FixedDotIndex, FixedDotWeight, GeneralDotWeight},
    },
    layout::LayoutEdit,
    math::Circle,
    router::{RouteStepper, Router},
    stepper::Step,
};

use super::{Autorouter, AutorouterError, AutorouterOptions};

pub struct PointrouteExecutionStepper {
    route: RouteStepper,
    options: AutorouterOptions,
}

impl PointrouteExecutionStepper {
    pub fn new(
        autorouter: &mut Autorouter<impl AccessMesadata>,
        origin: FixedDotIndex,
        point: Point,
        options: AutorouterOptions,
    ) -> Result<Self, AutorouterError> {
        let destination = autorouter.board.add_fixed_dot_infringably(
            &mut LayoutEdit::new(),
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: point,
                    r: options.router_options.routed_band_width / 2.0,
                },
                layer: 0,
                maybe_net: None,
            }),
            None,
        );

        let mut router = Router::new(autorouter.board.layout_mut(), options.router_options);

        Ok(Self {
            route: router.route(
                LayoutEdit::new(),
                origin,
                destination,
                options.router_options.routed_band_width,
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
        let mut router = Router::new(autorouter.board.layout_mut(), self.options.router_options);
        Ok(self.route.step(&mut router)?)
    }
}
