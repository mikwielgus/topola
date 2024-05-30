use std::collections::HashMap;

use geo::{point, Point, Rotate, Translate};
use thiserror::Error;

use crate::{
    autorouter::board::Board,
    drawing::{dot::FixedDotWeight, seg::FixedSegWeight, Drawing},
    dsn::{
        de,
        rules::DsnRules,
        structure::{self, DsnFile, Layer, Pcb, Shape},
    },
    geometry::compound::CompoundManagerTrait,
    graph::{GenericIndex, GetNodeIndex},
    layout::{zone::SolidZoneWeight, Layout},
    math::Circle,
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

    pub fn make_board(&self) -> Board<DsnRules> {
        let rules = DsnRules::from_pcb(&self.pcb);
        let mut board = Board::new(Layout::new(Drawing::new(rules)));

        // mapping of pin id -> net id prepared for adding pins
        let pin_nets = HashMap::<String, usize>::from_iter(
            self.pcb
                .network
                .net_vec
                .iter()
                .map(|net| {
                    // resolve the id so we don't work with strings
                    let net_id = board
                        .layout()
                        .drawing()
                        .rules()
                        .net_ids
                        .get(&net.name)
                        .unwrap();

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

                    for shape in padstack.shape_vec.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layer_vec,
                                    &circle.layer,
                                    place.side == "front",
                                );
                                Self::add_circle(
                                    &mut board,
                                    (place.x as f64, place.y as f64).into(),
                                    place.rotation as f64,
                                    (pin.x as f64, pin.y as f64).into(),
                                    pin.rotate.unwrap_or(0.0) as f64,
                                    circle.diameter as f64 / 2.0,
                                    layer as u64,
                                    *net_id,
                                    Some(pin_name.clone()),
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layer_vec,
                                    &rect.layer,
                                    place.side == "front",
                                );
                                Self::add_rect(
                                    &mut board,
                                    (place.x as f64, place.y as f64).into(),
                                    place.rotation as f64,
                                    (pin.x as f64, pin.y as f64).into(),
                                    pin.rotate.unwrap_or(0.0) as f64,
                                    rect.x1 as f64,
                                    rect.y1 as f64,
                                    rect.x2 as f64,
                                    rect.y2 as f64,
                                    layer as u64,
                                    *net_id,
                                    Some(pin_name.clone()),
                                )
                            }
                            Shape::Path(path) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layer_vec,
                                    &path.layer,
                                    place.side == "front",
                                );
                                Self::add_path(
                                    &mut board,
                                    (place.x as f64, place.y as f64).into(),
                                    place.rotation as f64,
                                    (pin.x as f64, pin.y as f64).into(),
                                    pin.rotate.unwrap_or(0.0) as f64,
                                    &path.coord_vec,
                                    path.width as f64,
                                    layer as u64,
                                    *net_id,
                                    Some(pin_name.clone()),
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = Self::layer(
                                    &mut board,
                                    &self.pcb.structure.layer_vec,
                                    &polygon.layer,
                                    place.side == "front",
                                );
                                Self::add_polygon(
                                    &mut board,
                                    (place.x as f64, place.y as f64).into(),
                                    place.rotation as f64,
                                    (pin.x as f64, pin.y as f64).into(),
                                    pin.rotate.unwrap_or(0.0) as f64,
                                    &polygon.coord_vec,
                                    polygon.width as f64,
                                    layer as u64,
                                    *net_id,
                                    Some(pin_name.clone()),
                                )
                            }
                        };
                    }
                }
            }
        }

        for via in &self.pcb.wiring.via_vec {
            let net_id = *board
                .layout()
                .drawing()
                .rules()
                .net_ids
                .get(&via.net)
                .unwrap();

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
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layer_vec,
                            &circle.layer,
                            true,
                        );
                        Self::add_circle(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            circle.diameter as f64 / 2.0,
                            layer as u64,
                            net_id,
                            None,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layer_vec,
                            &rect.layer,
                            true,
                        );
                        Self::add_rect(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            rect.x1 as f64,
                            rect.y1 as f64,
                            rect.x2 as f64,
                            rect.y2 as f64,
                            layer as u64,
                            net_id,
                            None,
                        )
                    }
                    Shape::Path(path) => {
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layer_vec,
                            &path.layer,
                            true,
                        );
                        Self::add_path(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            &path.coord_vec,
                            path.width as f64,
                            layer as u64,
                            net_id,
                            None,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = Self::layer(
                            &mut board,
                            &self.pcb.structure.layer_vec,
                            &polygon.layer,
                            true,
                        );
                        Self::add_polygon(
                            &mut board,
                            (0.0, 0.0).into(),
                            0.0,
                            (0.0, 0.0).into(),
                            0.0,
                            &polygon.coord_vec,
                            polygon.width as f64,
                            layer as u64,
                            net_id,
                            None,
                        )
                    }
                };
            }
        }

        for wire in self.pcb.wiring.wire_vec.iter() {
            let layer_id = *board
                .layout()
                .drawing()
                .rules()
                .layer_ids
                .get(&wire.path.layer)
                .unwrap();
            let net_id = *board
                .layout()
                .drawing()
                .rules()
                .net_ids
                .get(&wire.net)
                .unwrap();

            Self::add_path(
                &mut board,
                (0.0, 0.0).into(),
                0.0,
                (0.0, 0.0).into(),
                0.0,
                &wire.path.coord_vec,
                wire.path.width as f64,
                layer_id as u64,
                net_id,
                None,
            );
        }

        board
    }

    fn layer(
        board: &Board<DsnRules>,
        layer_vec: &Vec<Layer>,
        layer_name: &str,
        front: bool,
    ) -> usize {
        let image_layer = *board
            .layout()
            .drawing()
            .rules()
            .layer_ids
            .get(layer_name)
            .unwrap();

        if front {
            image_layer as usize
        } else {
            layer_vec.len() - image_layer as usize - 1
        }
    }

    fn add_circle(
        board: &mut Board<DsnRules>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        r: f64,
        layer: u64,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let circle = Circle {
            pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, 0.0, 0.0),
            r,
        };

        board
            .add_fixed_dot(
                FixedDotWeight {
                    circle,
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            )
            .unwrap();
    }

    fn add_rect(
        board: &mut Board<DsnRules>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: u64,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let zone = board.add_zone(
            SolidZoneWeight {
                layer,
                maybe_net: Some(net),
            }
            .into(),
            maybe_pin.clone(),
        );

        // Corners.
        let dot_1_1 = board
            .add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y1),
                        r: 0.5,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        let dot_2_1 = board
            .add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y1),
                        r: 0.5,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        let dot_2_2 = board
            .add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y2),
                        r: 0.5,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        let dot_1_2 = board
            .add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y2),
                        r: 0.5,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        // Sides.
        board
            .add_zone_fixed_seg(
                dot_1_1,
                dot_2_1,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        board
            .add_zone_fixed_seg(
                dot_2_1,
                dot_2_2,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        board
            .add_zone_fixed_seg(
                dot_2_2,
                dot_1_2,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
        board
            .add_zone_fixed_seg(
                dot_1_2,
                dot_1_1,
                FixedSegWeight {
                    width: 1.0,
                    layer,
                    maybe_net: Some(net),
                },
                zone,
            )
            .unwrap();
    }

    fn add_path(
        board: &mut Board<DsnRules>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        coords: &Vec<structure::Point>,
        width: f64,
        layer: u64,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_pos = Self::pos(
            place_pos,
            place_rot,
            pin_pos,
            pin_rot,
            coords[0].x as f64,
            coords[0].y as f64,
        );
        let mut prev_index = board
            .add_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: prev_pos,
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            )
            .unwrap();

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let pos = Self::pos(
                place_pos,
                place_rot,
                pin_pos,
                pin_rot,
                coord.x as f64,
                coord.y as f64,
            );

            if pos == prev_pos {
                continue;
            }

            let index = board
                .add_fixed_dot(
                    FixedDotWeight {
                        circle: Circle {
                            pos,
                            r: width / 2.0,
                        },
                        layer,
                        maybe_net: Some(net),
                    },
                    maybe_pin.clone(),
                )
                .unwrap();

            // add a seg between the current and previous coords
            let _ = board
                .add_fixed_seg(
                    prev_index,
                    index,
                    FixedSegWeight {
                        width,
                        layer,
                        maybe_net: Some(net),
                    },
                    maybe_pin.clone(),
                )
                .unwrap();

            prev_index = index;
            prev_pos = pos;
        }
    }

    fn add_polygon(
        board: &mut Board<DsnRules>,
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        coords: &Vec<structure::Point>,
        width: f64,
        layer: u64,
        net: usize,
        maybe_pin: Option<String>,
    ) {
        let zone = board.add_zone(
            SolidZoneWeight {
                layer,
                maybe_net: Some(net),
            }
            .into(),
            maybe_pin.clone(),
        );

        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_index = board
            .add_zone_fixed_dot(
                FixedDotWeight {
                    circle: Circle {
                        pos: Self::pos(
                            place_pos,
                            place_rot,
                            pin_pos,
                            pin_rot,
                            coords[0].x as f64,
                            coords[0].y as f64,
                        ),
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                //GenericIndex::new(zone.node_index()).into(),
                zone,
            )
            .unwrap();

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let index = board
                .add_zone_fixed_dot(
                    FixedDotWeight {
                        circle: Circle {
                            pos: Self::pos(
                                place_pos,
                                place_rot,
                                pin_pos,
                                pin_rot,
                                coord.x as f64,
                                coord.y as f64,
                            )
                            .into(),
                            r: width / 2.0,
                        },
                        layer,
                        maybe_net: Some(net),
                    },
                    // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                    zone,
                )
                .unwrap();

            // add a seg between the current and previous coords
            let _ = board
                .add_zone_fixed_seg(
                    prev_index,
                    index,
                    FixedSegWeight {
                        width,
                        layer,
                        maybe_net: Some(net),
                    },
                    // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                    zone,
                )
                .unwrap();

            prev_index = index;
        }
    }

    fn pos(
        place_pos: Point,
        place_rot: f64,
        pin_pos: Point,
        pin_rot: f64,
        x: f64,
        y: f64,
    ) -> Point {
        let pos = (point! {x: x, y: y} + pin_pos).rotate_around_point(pin_rot, pin_pos);
        (pos + place_pos).rotate_around_point(place_rot, place_pos)
    }
}
