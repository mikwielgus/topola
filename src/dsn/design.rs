use std::collections::HashMap;

use thiserror::Error;

use crate::{
    layout::{dot::FixedDotWeight, seg::FixedSegWeight, Layout},
    math::Circle,
};

use super::{
    de,
    rules::DsnRules,
    structure::{self, DsnFile, Pcb, Shape},
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
                            Shape::Circle(circle) => Self::add_circle(
                                &mut layout,
                                (place.x + pin.x) as f64,
                                -(place.y + pin.y) as f64,
                                circle.diameter as f64 / 2.0,
                                layer as u64,
                                *net_id as i64,
                            ),
                            Shape::Rect(rect) => Self::add_rect(
                                &mut layout,
                                (place.x - pin.x + rect.x1) as f64,
                                -(place.y - pin.y + rect.y1) as f64,
                                (place.x - pin.x + rect.x2) as f64,
                                -(place.y - pin.y + rect.y2) as f64,
                                layer as u64,
                                *net_id as i64,
                            ),
                            Shape::Path(path) => Self::add_path(
                                &mut layout,
                                (place.x - pin.x) as f64,
                                -(place.y - pin.y) as f64,
                                &path.coord_vec,
                                path.width as f64,
                                layer as u64,
                                *net_id as i64,
                            ),
                            Shape::Polygon(polygon) => Self::add_path(
                                &mut layout,
                                (place.x - pin.x) as f64,
                                -(place.y - pin.y) as f64,
                                &polygon.coord_vec,
                                polygon.width as f64,
                                layer as u64,
                                *net_id as i64,
                            ),
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

            for shape in &padstack.shape_vec {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = *layout.rules().layer_ids.get(&circle.layer).unwrap();

                        Self::add_circle(
                            &mut layout,
                            via.x as f64,
                            -via.y as f64,
                            circle.diameter as f64 / 2.0,
                            layer as u64,
                            net_id as i64,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = *layout.rules().layer_ids.get(&rect.layer).unwrap();
                        Self::add_rect(
                            &mut layout,
                            rect.x1 as f64,
                            rect.y1 as f64,
                            rect.x2 as f64,
                            rect.y2 as f64,
                            layer as u64,
                            net_id as i64,
                        )
                    }
                    Shape::Path(path) => {
                        let layer = *layout.rules().layer_ids.get(&path.layer).unwrap();
                        Self::add_path(
                            &mut layout,
                            0.0,
                            0.0,
                            &path.coord_vec,
                            path.width as f64,
                            layer as u64,
                            net_id as i64,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = *layout.rules().layer_ids.get(&polygon.layer).unwrap();
                        Self::add_path(
                            &mut layout,
                            0.0,
                            0.0,
                            &polygon.coord_vec,
                            polygon.width as f64,
                            layer as u64,
                            net_id as i64,
                        )
                    }
                };
            }
        }

        for wire in self.pcb.wiring.wire_vec.iter() {
            let layer_id = *layout.rules().layer_ids.get(&wire.path.layer).unwrap();
            let net_id = *layout.rules().net_ids.get(&wire.net).unwrap();

            Self::add_path(
                &mut layout,
                0.0,
                0.0,
                &wire.path.coord_vec,
                wire.path.width as f64,
                layer_id as u64,
                net_id as i64,
            );
        }

        layout
    }

    fn add_circle(layout: &mut Layout<DsnRules>, x: f64, y: f64, r: f64, layer: u64, net: i64) {
        let circle = Circle {
            pos: (x, y).into(),
            r,
        };

        layout
            .add_fixed_dot(FixedDotWeight { circle, layer, net })
            .unwrap();
    }

    fn add_rect(
        layout: &mut Layout<DsnRules>,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: u64,
        net: i64,
    ) {
        // Corners.
        let dot_1_1 = layout
            .add_fixed_dot(FixedDotWeight {
                circle: Circle {
                    pos: (x1, y1).into(),
                    r: 0.5,
                },
                layer,
                net,
            })
            .unwrap();
        let dot_2_1 = layout
            .add_fixed_dot(FixedDotWeight {
                circle: Circle {
                    pos: (x2, y1).into(),
                    r: 0.5,
                },
                layer,
                net,
            })
            .unwrap();
        let dot_2_2 = layout
            .add_fixed_dot(FixedDotWeight {
                circle: Circle {
                    pos: (x2, y2).into(),
                    r: 0.5,
                },
                layer,
                net,
            })
            .unwrap();
        let dot_1_2 = layout
            .add_fixed_dot(FixedDotWeight {
                circle: Circle {
                    pos: (x1, y2).into(),
                    r: 0.5,
                },
                layer,
                net,
            })
            .unwrap();
        // Sides.
        layout
            .add_fixed_seg(
                dot_1_1,
                dot_2_1,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    net,
                },
            )
            .unwrap();
        layout
            .add_fixed_seg(
                dot_2_1,
                dot_2_2,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    net,
                },
            )
            .unwrap();
        layout
            .add_fixed_seg(
                dot_2_2,
                dot_1_2,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    net,
                },
            )
            .unwrap();
        layout
            .add_fixed_seg(
                dot_1_2,
                dot_1_1,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    net,
                },
            )
            .unwrap();
    }

    fn add_path(
        layout: &mut Layout<DsnRules>,
        offset_x: f64,
        offset_y: f64,
        coords: &Vec<structure::Point>,
        width: f64,
        layer: u64,
        net: i64,
    ) {
        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_index = layout
            .add_fixed_dot(FixedDotWeight {
                circle: Circle {
                    pos: (offset_x + coords[0].x as f64, offset_y - coords[0].y as f64).into(),
                    r: width / 2.0,
                },
                layer,
                net,
            })
            .unwrap();

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let index = layout
                .add_fixed_dot(FixedDotWeight {
                    circle: Circle {
                        pos: (offset_x + coord.x as f64, offset_y - coord.y as f64).into(),
                        r: width / 2.0,
                    },
                    layer,
                    net,
                })
                .unwrap();

            // add a seg between the current and previous coords
            let _ = layout
                .add_fixed_seg(prev_index, index, FixedSegWeight { width, layer, net })
                .unwrap();

            prev_index = index;
        }
    }
}
