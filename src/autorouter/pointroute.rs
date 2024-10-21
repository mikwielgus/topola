use std::ops::ControlFlow;

use geo::Point;

use crate::{
    board::mesadata::AccessMesadata,
    drawing::{
        band::BandTermsegIndex,
        dot::{FixedDotIndex, FixedDotWeight},
    },
    math::Circle,
    router::{route::RouteStepper, Router},
    stepper::Step,
};

use super::{Autorouter, AutorouterError, AutorouterOptions};

pub struct PointrouteExecutionStepper {
    point: Point,
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
            FixedDotWeight {
                circle: Circle {
                    pos: point,
                    r: options.router_options.routed_band_width / 2.0,
                },
                layer: 0,
                maybe_net: None,
            },
            None,
        );

        let mut router = Router::new(autorouter.board.layout_mut(), options.router_options);

        Ok(Self {
            point,
            route: router.route(
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
