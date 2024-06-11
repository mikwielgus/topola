use std::collections::HashMap;

use geo::{point, Point, Rotate, Translate};
use thiserror::Error;

use crate::{
    board::{mesadata::MesadataTrait, Board},
    drawing::{dot::FixedDotWeight, seg::FixedSegWeight, Drawing},
    dsn::{
        de,
        mesadata::DsnMesadata,
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

    pub fn make_board(&self) -> Board<DsnMesadata> {
        let rules = DsnMesadata::from_pcb(&self.pcb);
        let mut board = Board::new(Layout::new(Drawing::new(rules)));

        // mapping of pin -> net prepared for adding pins
        let pin_nets = HashMap::<String, usize>::from_iter(
            self.pcb
                .network
                .net_vec
                .iter()
                .map(|net_pin_assignments| {
                    // resolve the id so we don't work with strings
                    let net = board
                        .layout()
                        .drawing()
                        .rules()
                        .netname_net(&net_pin_assignments.name)
                        .unwrap();

                    // take the list of pins
                    // and for each pin id output (pin id, net id)
                    net_pin_assignments
                        .pins
                        .names
                        .iter()
                        .map(move |pinname| (pinname.clone(), net))
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
                    let pinname = format!("{}-{}", place.name, pin.id);
                    let net = pin_nets.get(&pinname).unwrap();

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
                                    *net,
                                    Some(pinname.clone()),
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
                                    *net,
                                    Some(pinname.clone()),
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
                                    *net,
                                    Some(pinname.clone()),
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
                                    *net,
                                    Some(pinname.clone()),
                                )
                            }
                        };
                    }
                }
            }
        }

        for via in &self.pcb.wiring.via_vec {
            let net = board
                .layout()
                .drawing()
                .rules()
                .netname_net(&via.net)
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
                            net,
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
                            net,
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
                            net,
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
                            net,
                            None,
                        )
                    }
                };
            }
        }

        for wire in self.pcb.wiring.wire_vec.iter() {
            let layer = board
                .layout()
                .drawing()
                .rules()
                .layername_layer(&wire.path.layer)
                .unwrap();
            let net = board
                .layout()
                .drawing()
                .rules()
                .netname_net(&wire.net)
                .unwrap();

            Self::add_path(
                &mut board,
                (0.0, 0.0).into(),
                0.0,
                (0.0, 0.0).into(),
                0.0,
                &wire.path.coord_vec,
                wire.path.width as f64,
                layer as u64,
                net,
                None,
            );
        }

        // The clones here are bad, we'll have something better later on.

        let layername_to_layer = &board.layout().drawing().rules().layer_layername.clone();

        for (layer, layername) in layername_to_layer.iter() {
            board
                .layout_mut()
                .rules_mut()
                .bename_layer(*layer, layername.to_string());
        }

        let netname_to_net = &board.layout().drawing().rules().net_netname.clone();

        for (net, netname) in netname_to_net.iter() {
            board
                .layout_mut()
                .rules_mut()
                .bename_net(*net, netname.to_string());
        }

        board
    }

    fn layer(
        board: &Board<DsnMesadata>,
        layer_vec: &Vec<Layer>,
        layername: &str,
        front: bool,
    ) -> usize {
        let image_layer = board
            .layout()
            .drawing()
            .rules()
            .layername_layer(layername)
            .unwrap();

        if front {
            image_layer as usize
        } else {
            layer_vec.len() - image_layer as usize - 1
        }
    }

    fn add_circle(
        board: &mut Board<DsnMesadata>,
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

        board.add_fixed_dot_infringably(
            FixedDotWeight {
                circle,
                layer,
                maybe_net: Some(net),
            },
            maybe_pin.clone(),
        );
    }

    fn add_rect(
        board: &mut Board<DsnMesadata>,
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
        let dot_1_1 = board.add_zone_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y1),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        let dot_2_1 = board.add_zone_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y1),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        let dot_2_2 = board.add_zone_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x2, y2),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        let dot_1_2 = board.add_zone_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: Self::pos(place_pos, place_rot, pin_pos, pin_rot, x1, y2),
                    r: 0.5,
                },
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        // Sides.
        board.add_zone_fixed_seg_infringably(
            dot_1_1,
            dot_2_1,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        board.add_zone_fixed_seg_infringably(
            dot_2_1,
            dot_2_2,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        board.add_zone_fixed_seg_infringably(
            dot_2_2,
            dot_1_2,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
        board.add_zone_fixed_seg_infringably(
            dot_1_2,
            dot_1_1,
            FixedSegWeight {
                width: 1.0,
                layer,
                maybe_net: Some(net),
            },
            zone,
        );
    }

    fn add_path(
        board: &mut Board<DsnMesadata>,
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
        let mut prev_index = board.add_fixed_dot_infringably(
            FixedDotWeight {
                circle: Circle {
                    pos: prev_pos,
                    r: width / 2.0,
                },
                layer,
                maybe_net: Some(net),
            },
            maybe_pin.clone(),
        );

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

            let index = board.add_fixed_dot_infringably(
                FixedDotWeight {
                    circle: Circle {
                        pos,
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            );

            // add a seg between the current and previous coords
            let _ = board.add_fixed_seg_infringably(
                prev_index,
                index,
                FixedSegWeight {
                    width,
                    layer,
                    maybe_net: Some(net),
                },
                maybe_pin.clone(),
            );

            prev_index = index;
            prev_pos = pos;
        }
    }

    fn add_polygon(
        board: &mut Board<DsnMesadata>,
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
        let mut prev_index = board.add_zone_fixed_dot_infringably(
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
        );

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let index = board.add_zone_fixed_dot_infringably(
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
            );

            // add a seg between the current and previous coords
            let _ = board.add_zone_fixed_seg_infringably(
                prev_index,
                index,
                FixedSegWeight {
                    width,
                    layer,
                    maybe_net: Some(net),
                },
                // TODO: This manual retagging shouldn't be necessary, `.into()` should suffice.
                zone,
            );

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
