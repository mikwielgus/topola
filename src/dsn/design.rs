use std::collections::HashMap;

use thiserror::Error;

use crate::{
    layout::{dot::FixedDotWeight, seg::FixedSegWeight, Layout},
    math::Circle,
};

use super::{
    de,
    rules::DsnRules,
    structure::{DsnFile, Pcb, Shape},
};

#[derive(Error, Debug)]
pub enum LoadingError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Syntax(#[from] de::SyntaxError),
}

#[derive(Debug)]
pub struct DsnDesign {
    pcb: Pcb,
}

impl DsnDesign {
    pub fn load_from_file(filename: &str) -> Result<Self, LoadingError> {
        let contents = std::fs::read_to_string(filename)?;
        Self::load_from_string(contents)
    }

    pub fn load_from_string(contents: String) -> Result<Self, LoadingError> {
        let pcb = de::from_str::<DsnFile>(&contents)
            .map_err(|err| LoadingError::Syntax(err))?
            .pcb;

        Ok(Self { pcb })
    }

    pub fn make_layout(&self) -> Layout<DsnRules> {
        let rules = DsnRules::from_pcb(&self.pcb);
        let mut layout = Layout::new(rules);

        // mapping of pin id -> net id prepared for adding pins
        let pin_nets = HashMap::<String, i64>::from_iter(
            self.pcb
                .network
                .net_vec
                .iter()
                .map(|net| {
                    // resolve the id so we don't work with strings
                    let net_id = layout.rules().net_ids.get(&net.name).unwrap();

                    // take the list of pins
                    // and for each pin id output (pin id, net id)
                    net.pins.names.iter().map(|id| (id.clone(), *net_id))
                })
                // flatten the nested iters into a single stream of tuples
                .flatten(),
        );

        // add pins from components
        for component in &self.pcb.placement.component_vec {
            for place in &component.place_vec {
                let image = self
                    .pcb
                    .library
                    .image_vec
                    .iter()
                    .find(|image| image.name == component.name)
                    .unwrap();

                for pin in &image.pin_vec {
                    let pin_name = format!("{}-{}", place.name, pin.id);
                    let net_id = pin_nets.get(&pin_name).unwrap();

                    let padstack = &self
                        .pcb
                        .library
                        .padstack_vec
                        .iter()
                        .find(|padstack| padstack.name == pin.name)
                        .unwrap();

                    for (layer, shape) in padstack.shape_vec.iter().enumerate() {
                        match shape {
                            Shape::Circle(circle) => {
                                let circle = Circle {
                                    pos: ((place.x + pin.x) as f64, -(place.y + pin.y) as f64)
                                        .into(),
                                    r: circle.diameter as f64 / 2.0,
                                };

                                layout
                                    .add_fixed_dot(FixedDotWeight {
                                        circle,
                                        layer: layer as u64,
                                        net: *net_id as i64,
                                    })
                                    .unwrap();
                            }
                            Shape::Rect(_) => (),
                            Shape::Path(_) => (),
                            Shape::Polygon(_) => (),
                        };
                    }
                }
            }
        }

        for via in &self.pcb.wiring.via_vec {
            let net_id = *layout.rules().net_ids.get(&via.net).unwrap();

            // find the padstack referenced by this via placement
            let padstack = &self
                .pcb
                .library
                .padstack_vec
                .iter()
                .find(|padstack| padstack.name == via.name)
                .unwrap();

            // no layer support yet, pick the first one
            for shape in &padstack.shape_vec {
                let circle = match shape {
                    Shape::Circle(circle) => circle,
                    Shape::Rect(_) => todo!(),
                    Shape::Path(_) => todo!(),
                    Shape::Polygon(_) => todo!(),
                };
                let layer_id = *layout.rules().layer_ids.get(&circle.layer).unwrap();
                let circle = Circle {
                    pos: (via.x as f64, -via.y as f64).into(),
                    r: circle.diameter as f64 / 2.0,
                };

                layout.add_fixed_dot(FixedDotWeight {
                    circle,
                    layer: layer_id as u64,
                    net: net_id as i64,
                });
            }
        }

        for wire in self.pcb.wiring.wire_vec.iter() {
            let layer_id = *layout.rules().layer_ids.get(&wire.path.layer).unwrap();
            let net_id = *layout.rules().net_ids.get(&wire.net).unwrap();

            // add the first coordinate in the wire path as a dot and save its index
            let mut prev_index = layout
                .add_fixed_dot(FixedDotWeight {
                    circle: Circle {
                        pos: (
                            wire.path.coord_vec[0].x as f64,
                            -wire.path.coord_vec[0].y as f64,
                        )
                            .into(),
                        r: wire.path.width as f64 / 2.0,
                    },
                    layer: layer_id as u64,
                    net: net_id as i64,
                })
                .unwrap();

            // iterate through path coords starting from the second
            for coord in wire.path.coord_vec.iter().skip(1) {
                let index = layout
                    .add_fixed_dot(FixedDotWeight {
                        circle: Circle {
                            pos: (coord.x as f64, -coord.y as f64).into(),
                            r: wire.path.width as f64 / 2.0,
                        },
                        layer: layer_id as u64,
                        net: net_id as i64,
                    })
                    .unwrap();

                // add a seg between the current and previous coords
                let _ = layout
                    .add_fixed_seg(
                        prev_index,
                        index,
                        FixedSegWeight {
                            width: wire.path.width as f64,
                            layer: layer_id as u64,
                            net: net_id as i64,
                        },
                    )
                    .unwrap();

                prev_index = index;
            }
        }

        layout
    }
}
