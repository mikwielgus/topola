use std::{collections::HashMap, fmt};

use serde::Deserialize;
use thiserror::Error;

use crate::{
    layout::{dot::FixedDotWeight, seg::FixedSegWeight, Layout},
    math::Circle,
};

use super::{
    de::{Deserializer, DsnContext, DsnDeError},
    rules::Rules,
    structure::{DsnFile, Pcb, Shape},
};

#[derive(Error, Debug)]
pub struct DsnError(DsnContext, DsnDeError);

impl fmt::Display for DsnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{0}: {1}", self.0, self.1)
    }
}

#[derive(Debug)]
pub struct DsnDesign {
    pcb: Pcb,
    rules: Rules,
}

impl DsnDesign {
    pub fn load_from_file(filename: &str) -> Result<Self, DsnError> {
        let contents = std::fs::read_to_string(filename).unwrap(); // TODO: remove unwrap.
                                                                   //let pcb = from_str::<Pcb>(&contents).map_err(|err| DsnError())?;
        let mut deserializer = Deserializer::from_str(&contents);
        let pcb = DsnFile::deserialize(&mut deserializer)
            .map_err(|err| DsnError(deserializer.context, err))?.pcb;

        let rules = Rules::from_pcb(&pcb);

        Ok(Self { pcb, rules })
    }

    pub fn make_layout(&self) -> Layout<&Rules> {
        let mut layout = Layout::new(&self.rules);

        // mapping of pin id -> net id prepared for adding pins
        let pin_nets = HashMap::<String, i64>::from_iter(
            self.pcb.network.net_vec
                .iter()
                .map(|net| {
                    // resolve the id so we don't work with strings
                    let net_id = self.rules.net_ids.get(&net.name).unwrap();

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

                    // no layer support yet, pick the first one
                    match &padstack.shape_vec[0] {
                        Shape::Circle(circle) => {
                            let circle = Circle {
                                pos: (
                                    (place.x + pin.x) as f64 / 100.0,
                                    -(place.y + pin.y) as f64 / 100.0
                                ).into(),
                                r: circle.diameter as f64 / 200.0,
                            };

                            layout
                                .add_fixed_dot(FixedDotWeight {
                                    net: *net_id as i64,
                                    circle,
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

        // add vias to layout and save indices of dots in the order they appear in the file
        let _dot_indices: Vec<_> = self
            .pcb
            .wiring
            .via_vec
            .iter()
            .map(|via| {
                let net_id = self.rules.net_ids.get(&via.net).unwrap();

                // find the padstack referenced by this via placement
                let padstack = &self
                    .pcb
                    .library
                    .padstack_vec
                    .iter()
                    .find(|padstack| padstack.name == via.name)
                    .unwrap();

                // no layer support yet, pick the first one
                let circle = match &padstack.shape_vec[0] {
                    Shape::Circle(circle) => circle,
                    Shape::Rect(_) => todo!(),
                    Shape::Path(_) => todo!(),
                    Shape::Polygon(_) => todo!(),
                };
                let circle = Circle {
                    pos: (via.x as f64 / 100.0, -via.y as f64 / 100.0).into(),
                    r: circle.diameter as f64 / 200.0,
                };

                layout
                    .add_fixed_dot(FixedDotWeight {
                        net: *net_id as i64,
                        circle,
                    })
                    .unwrap()
            })
            .collect();

        for wire in self.pcb.wiring.wire_vec.iter() {
            let net_id = self.rules.net_ids.get(&wire.net).unwrap();

            // add the first coordinate in the wire path as a dot and save its index
            let mut prev_index = layout
                .add_fixed_dot(FixedDotWeight {
                    net: *net_id as i64,
                    circle: Circle {
                        pos: (
                            wire.path.coord_vec[0].x as f64 / 100.0,
                            -wire.path.coord_vec[0].y as f64 / 100.0,
                        )
                            .into(),
                        r: wire.path.width as f64 / 200.0,
                    },
                })
                .unwrap();

            // iterate through path coords starting from the second
            for coord in wire.path.coord_vec.iter().skip(1) {
                let index = layout
                    .add_fixed_dot(FixedDotWeight {
                        net: *net_id as i64,
                        circle: Circle {
                            pos: (coord.x as f64 / 100.0, -coord.y as f64 / 100.0).into(),
                            r: wire.path.width as f64 / 200.0,
                        },
                    })
                    .unwrap();

                // add a seg between the current and previous coords
                let _ = layout
                    .add_fixed_seg(
                        prev_index,
                        index,
                        FixedSegWeight {
                            net: *net_id as i64,
                            width: wire.path.width as f64 / 100.0,
                        },
                    )
                    .unwrap();

                prev_index = index;
            }
        }

        layout
    }
}
