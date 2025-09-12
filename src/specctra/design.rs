// SPDX-FileCopyrightText: 2024 Topola contributors
//
// SPDX-License-Identifier: MIT

//! Module for managing the various Specctra PCB design, including loading the
//! Design DSN file, creating the [`Board`] object from the file, as well as
//! exporting the session file

use std::collections::{btree_map::Entry as BTreeMapEntry, BTreeMap};

use geo::{Euclidean, Length, Line, Point, Rotate};
use itertools::Itertools;
use specctra_core::math::PointWithRotation;

use crate::{
    board::{edit::BoardEdit, AccessMesadata, Board},
    drawing::{
        dot::{FixedDotIndex, FixedDotWeight, GeneralDotWeight},
        graph::{GetMaybeNet, MakePrimitiveRef},
        primitive::MakePrimitiveShape,
        seg::{FixedSegWeight, GeneralSegWeight},
        Drawing,
    },
    geometry::{primitive::PrimitiveShape, GetLayer, GetWidth},
    layout::{poly::SolidPolyWeight, Layout},
    math::{self, Circle},
    specctra::{
        mesadata::SpecctraMesadata,
        read::ListTokenizer,
        structure::{self, DsnFile, Layer, Pcb, Shape},
        write::ListWriter,
    },
};

pub use specctra_core::error::ParseErrorContext;

/// This struct is responsible for managing the various Specctra components of a PCB design,
/// including parsing the DSN file, handling the resolution, unit of measurement,
/// and organizing the PCB's structure, placement, library, network, and wiring.
/// It provides functionality for reading from a DSN file and writing Specctra's .SES session files.
#[derive(Debug)]
pub struct SpecctraDesign {
    pcb: Pcb,
}

impl SpecctraDesign {
    /// Loads a [`SpecctraDesign`] structure instance from a buffered reader.
    ///
    /// This function reads the Specctra Design data from an input stream.
    /// Later the data is parsed and loaded into a [`SpecctraDesign`] structure,
    /// allowing further operations such as rule validation, routing, or netlist management.
    pub fn load(reader: impl std::io::BufRead) -> Result<SpecctraDesign, ParseErrorContext> {
        let mut list_reader = ListTokenizer::new(reader);
        let dsn = list_reader.read_value::<DsnFile>()?;

        Ok(Self { pcb: dsn.pcb })
    }

    /// Function to get name of the DSN file
    ///
    /// This function returns the name of the `Pcb` objects
    pub fn get_name(&self) -> &str {
        &self.pcb.name
    }

    /// Writes the Specctra Session (.ses) file format using the current board layout and mesadata.
    ///
    /// This function generates a Specctra SES session file that represents the board's net routing and
    /// writes it to the provided output stream. The session data includes routed nets, wires,
    /// layers, and other essential information for routing management.
    pub fn write_ses(
        &self,
        board: &Board<SpecctraMesadata>,
        writer: impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        let mesadata = board.mesadata();
        let drawing = board.layout().drawing();

        let mut net_outs = BTreeMap::<usize, structure::NetOut>::new();
        for index in drawing.primitive_nodes() {
            let primitive = index.primitive_ref(drawing);

            if let Some(net) = primitive.maybe_net() {
                let coords = match primitive.shape() {
                    PrimitiveShape::Seg(seg) => {
                        vec![
                            structure::Point {
                                x: seg.from.x(),
                                y: seg.from.y(),
                            },
                            structure::Point {
                                x: seg.to.x(),
                                y: seg.to.y(),
                            },
                        ]
                    }

                    PrimitiveShape::Bend(bend) => {
                        // Since general circle arcs don't seem to be supported
                        // we're downgrading each one to a chain of straight
                        // line segments.
                        // TODO: make this configurable? pick a smarter value?
                        let segment_count: usize = 100;
                        bend.render_discretization(segment_count + 1)
                            .map(Into::into)
                            .collect()
                    }

                    // Intentionally skipped for now.
                    // Topola stores trace segments and dots joining them
                    // as separate objects, but the Specctra formats and KiCad
                    // appear to consider them implicit.
                    // TODO: Vias
                    PrimitiveShape::Dot(_) => continue,
                };

                let wire = structure::WireOut {
                    path: structure::Path {
                        layer: mesadata
                            .layer_layername(primitive.layer())
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!(
                                        "tried to reference invalid primitive layer {}",
                                        primitive.layer()
                                    ),
                                )
                            })?
                            .to_owned(),
                        width: primitive.width(),
                        coords,
                    },
                };

                let net_out = match net_outs.entry(net) {
                    BTreeMapEntry::Occupied(occ) => occ.into_mut(),
                    BTreeMapEntry::Vacant(vac) => vac.insert(structure::NetOut {
                        name: mesadata
                            .net_netname(net)
                            .ok_or_else(|| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("tried to reference invalid net ID {}", net),
                                )
                            })?
                            .to_owned(),
                        wire: Vec::new(),
                        via: Vec::new(),
                    }),
                };
                net_out.wire.push(wire);
            }
        }

        let ses = structure::SesFile {
            session: structure::Session {
                id: "ID".to_string(),
                routes: structure::Routes {
                    resolution: structure::Resolution {
                        unit: self.pcb.resolution.unit.clone(),
                        value: 1.0,
                    },
                    library_out: structure::Library {
                        images: Vec::new(),
                        padstacks: Vec::new(),
                    },
                    network_out: structure::NetworkOut {
                        net: net_outs.into_values().collect(),
                    },
                },
            },
        };

        ListWriter::new(writer).write_value(&ses)
    }

    /// Generates a [`Board<SpecctraMesadata>`] from the current PCB data.
    ///
    /// This function takes the internal `Pcb` structure and transforms it into a [`Board`] object,
    /// which is used for layout and routing operations. The board is initialized with [`SpecctraMesadata`],
    /// which includes layer and net mappings, and is populated with components, pins, vias, and wires
    /// from the PCB definition.
    pub fn make_board(&self, recorder: &mut BoardEdit) -> Board<SpecctraMesadata> {
        let mesadata = SpecctraMesadata::from_pcb(&self.pcb);
        let mut board = Board::new(Layout::new(Drawing::new(
            mesadata,
            self.pcb.structure.layers.len(),
            self.pcb.structure.boundary.to_polygon(),
            self.pcb
                .structure
                .place_boundary
                .as_ref()
                .map(|i| i.to_polygon()),
        )));

        // mapping of pin -> net prepared for adding pins
        let pin_nets = self
            .pcb
            .network
            .nets
            .iter()
            .filter_map(|net_pin_assignments| {
                // resolve the id so we don't work with strings
                let net = board
                    .layout()
                    .drawing()
                    .rules()
                    .netname_net(&net_pin_assignments.name)
                    .unwrap();

                net_pin_assignments.pins.as_ref().map(|pins| {
                    // take the list of pins
                    // and for each pin output (pin name, net id)
                    pins.names.iter().map(move |pinname| (pinname.clone(), net))
                })
            })
            // flatten the nested iters into a single stream of tuples
            .flatten()
            .collect::<BTreeMap<String, usize>>();

        // add pins from components
        for component in &self.pcb.placement.components {
            let image = self
                .pcb
                .library
                .images
                .iter()
                .find(|image| image.name == component.name)
                .unwrap();

            for place in &component.places {
                let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board<SpecctraMesadata>, name: &str| {
                    Self::layer(board, &self.pcb.structure.layers, name, place_side_is_front)
                };

                for pin in &image.pins {
                    let pinname = format!("{}-{}", place.name, pin.id);
                    let net = pin_nets.get(&pinname).copied();

                    let padstack = self.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = get_layer(&board, &circle.layer);
                                Self::add_circle(
                                    recorder,
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    circle.diameter / 2.0,
                                    layer,
                                    net,
                                    Some(pinname.clone()),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = get_layer(&board, &rect.layer);
                                Self::add_rect(
                                    recorder,
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    rect.x1,
                                    rect.y1,
                                    rect.x2,
                                    rect.y2,
                                    layer,
                                    net,
                                    Some(pinname.clone()),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Path(path) => {
                                let layer = get_layer(&board, &path.layer);
                                Self::add_path(
                                    recorder,
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &path.coords,
                                    path.width,
                                    layer,
                                    net,
                                    Some(pinname.clone()),
                                    !place_side_is_front,
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = get_layer(&board, &polygon.layer);
                                Self::add_polygon(
                                    recorder,
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &polygon.coords,
                                    polygon.width,
                                    layer,
                                    net,
                                    Some(pinname.clone()),
                                    !place_side_is_front,
                                )
                            }
                        };
                    }
                }
            }
        }

        for via in &self.pcb.wiring.vias {
            let net = board.layout().drawing().rules().netname_net(&via.net);

            let padstack = self.pcb.library.find_padstack_by_name(&via.name).unwrap();

            let get_layer = |board: &Board<SpecctraMesadata>, name: &str| {
                Self::layer(board, &self.pcb.structure.layers, name, true)
            };

            for shape in &padstack.shapes {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = get_layer(&board, &circle.layer);
                        Self::add_circle(
                            recorder,
                            &mut board,
                            // TODO: refactor?
                            // should this call take PointWithRotation?
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            circle.diameter / 2.0,
                            layer,
                            net,
                            None,
                            false,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = get_layer(&board, &rect.layer);
                        Self::add_rect(
                            recorder,
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            rect.x1,
                            rect.y1,
                            rect.x2,
                            rect.y2,
                            layer,
                            net,
                            None,
                            false,
                        )
                    }
                    Shape::Path(path) => {
                        let layer = get_layer(&board, &path.layer);
                        Self::add_path(
                            recorder,
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &path.coords,
                            path.width,
                            layer,
                            net,
                            None,
                            false,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = get_layer(&board, &polygon.layer);
                        Self::add_polygon(
                            recorder,
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &polygon.coords,
                            polygon.width,
                            layer,
                            net,
                            None,
                            false,
                        )
                    }
                };
            }
        }

        for wire in self.pcb.wiring.wires.iter() {
            let layer = board
                .layout()
                .drawing()
                .rules()
                .layername_layer(&wire.path.layer)
                .unwrap();
            let net = board.layout().drawing().rules().netname_net(&wire.net);

            Self::add_path(
                recorder,
                &mut board,
                PointWithRotation::default(),
                PointWithRotation::default(),
                &wire.path.coords,
                wire.path.width,
                layer,
                net,
                None,
                false,
            );
        }

        board
    }

    fn layer(
        board: &Board<SpecctraMesadata>,
        layers: &[Layer],
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
            image_layer
        } else {
            layers.len() - image_layer - 1
        }
    }

    fn add_circle(
        recorder: &mut BoardEdit,
        board: &mut Board<SpecctraMesadata>,
        place: PointWithRotation,
        pin: PointWithRotation,
        r: f64,
        layer: usize,
        maybe_net: Option<usize>,
        maybe_pin: Option<String>,
        flip: bool,
    ) {
        let circle = Circle {
            pos: Self::pos(place, pin, 0.0, 0.0, flip),
            r,
        };

        board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle,
                layer,
                maybe_net,
            }),
            maybe_pin,
        );
    }

    fn add_rect(
        recorder: &mut BoardEdit,
        board: &mut Board<SpecctraMesadata>,
        place: PointWithRotation,
        pin: PointWithRotation,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: usize,
        maybe_net: Option<usize>,
        maybe_pin: Option<String>,
        flip: bool,
    ) {
        // Corners.
        let dot_1_1 = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: Self::pos(place, pin, x1, y1, flip),
                    r: 0.5,
                },
                layer,
                maybe_net,
            }),
            None,
        );
        let dot_2_1 = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: Self::pos(place, pin, x2, y1, flip),
                    r: 0.5,
                },
                layer,
                maybe_net,
            }),
            None,
        );
        let dot_2_2 = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: Self::pos(place, pin, x2, y2, flip),
                    r: 0.5,
                },
                layer,
                maybe_net,
            }),
            None,
        );
        let dot_1_2 = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: Self::pos(place, pin, x1, y2, flip),
                    r: 0.5,
                },
                layer,
                maybe_net,
            }),
            None,
        );
        // Sides.
        let seg1 = board.add_fixed_seg_infringably(
            recorder,
            dot_1_1,
            dot_2_1,
            FixedSegWeight(GeneralSegWeight {
                width: 1.0,
                layer,
                maybe_net,
            }),
            None,
        );
        let seg2 = board.add_fixed_seg_infringably(
            recorder,
            dot_2_1,
            dot_2_2,
            FixedSegWeight(GeneralSegWeight {
                width: 1.0,
                layer,
                maybe_net,
            }),
            None,
        );
        let seg3 = board.add_fixed_seg_infringably(
            recorder,
            dot_2_2,
            dot_1_2,
            FixedSegWeight(GeneralSegWeight {
                width: 1.0,
                layer,
                maybe_net,
            }),
            None,
        );
        let seg4 = board.add_fixed_seg_infringably(
            recorder,
            dot_1_2,
            dot_1_1,
            FixedSegWeight(GeneralSegWeight {
                width: 1.0,
                layer,
                maybe_net,
            }),
            None,
        );

        board.add_poly_with_nodes(
            recorder,
            SolidPolyWeight { layer, maybe_net }.into(),
            maybe_pin,
            &[
                dot_1_1.into(),
                dot_1_2.into(),
                dot_2_2.into(),
                dot_2_1.into(),
                seg1.into(),
                seg2.into(),
                seg3.into(),
                seg4.into(),
            ],
            &[],
        );
    }

    fn add_path(
        recorder: &mut BoardEdit,
        board: &mut Board<SpecctraMesadata>,
        place: PointWithRotation,
        pin: PointWithRotation,
        coords: &[structure::Point],
        width: f64,
        layer: usize,
        maybe_net: Option<usize>,
        maybe_pin: Option<String>,
        flip: bool,
    ) {
        // add the first coordinate in the wire path as a dot and save its index
        let mut prev_pos = Self::pos(place, pin, coords[0].x, coords[0].y, flip);
        let mut prev_index = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: prev_pos,
                    r: width / 2.0,
                },
                layer,
                maybe_net,
            }),
            maybe_pin.clone(),
        );

        // iterate through path coords starting from the second
        for coord in coords.iter().skip(1) {
            let pos = Self::pos(place, pin, coord.x, coord.y, flip);

            if pos == prev_pos {
                continue;
            }

            let index = board.add_fixed_dot_infringably(
                recorder,
                FixedDotWeight(GeneralDotWeight {
                    circle: Circle {
                        pos,
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net,
                }),
                maybe_pin.clone(),
            );

            // add a seg between the current and previous coords
            let _ = board.add_fixed_seg_infringably(
                recorder,
                prev_index,
                index,
                FixedSegWeight(GeneralSegWeight {
                    width,
                    layer,
                    maybe_net,
                }),
                maybe_pin.clone(),
            );

            prev_index = index;
            prev_pos = pos;
        }
    }

    fn add_polygon(
        recorder: &mut BoardEdit,
        board: &mut Board<SpecctraMesadata>,
        place: PointWithRotation,
        pin: PointWithRotation,
        mut coords: &[structure::Point],
        width: f64,
        layer: usize,
        maybe_net: Option<usize>,
        maybe_pin: Option<String>,
        flip: bool,
    ) {
        let mut nodes = Vec::with_capacity(coords.len() * 2 - 1);

        // add the first coordinate in the wire path as a dot and save its index
        let first_index = board.add_fixed_dot_infringably(
            recorder,
            FixedDotWeight(GeneralDotWeight {
                circle: Circle {
                    pos: Self::pos(place, pin, coords[0].x, coords[0].y, flip),
                    r: width / 2.0,
                },
                layer,
                maybe_net,
            }),
            None,
        );
        nodes.push(first_index.into());
        let mut prev_index = first_index;

        if approx::abs_diff_eq!(coords[0].x, coords.last().unwrap().x)
            && approx::abs_diff_eq!(coords[0].y, coords.last().unwrap().y)
        {
            coords = &coords[..coords.len() - 1];
        }

        let seg_weight = FixedSegWeight(GeneralSegWeight {
            width,
            layer,
            maybe_net,
        });

        // iterate through path coords starting from the second
        for coord in &coords[1..] {
            let index = board.add_fixed_dot_infringably(
                recorder,
                FixedDotWeight(GeneralDotWeight {
                    circle: Circle {
                        pos: Self::pos(place, pin, coord.x, coord.y, flip),
                        r: width / 2.0,
                    },
                    layer,
                    maybe_net,
                }),
                None,
            );
            nodes.push(index.into());

            // add a seg between the current and previous coords
            nodes.push(
                board
                    .add_fixed_seg_infringably(recorder, prev_index, index, seg_weight, None)
                    .into(),
            );

            prev_index = index;
        }

        // add a seg between the last and first coords
        nodes.push(
            board
                .add_fixed_seg_infringably(recorder, prev_index, first_index, seg_weight, None)
                .into(),
        );

        let fillets = Self::add_polygon_fillet_circles(
            recorder, board, place, pin, coords, width, layer, maybe_net, None, flip,
        );

        board.add_poly_with_nodes(
            recorder,
            SolidPolyWeight { layer, maybe_net }.into(),
            maybe_pin,
            &nodes[..],
            &fillets[..],
        );
    }

    fn add_polygon_fillet_circles(
        recorder: &mut BoardEdit,
        board: &mut Board<SpecctraMesadata>,
        place: PointWithRotation,
        pin: PointWithRotation,
        coords: &[structure::Point],
        _width: f64,
        layer: usize,
        maybe_net: Option<usize>,
        _maybe_pin: Option<String>,
        flip: bool,
    ) -> Vec<FixedDotIndex> {
        let MIN_FIRST_CHAIN_ELEMENT_LENGTH = 100.0;
        let mut maybe_first_chain_segment = None;
        let mut fillets = vec![];

        let first_pos = Self::pos(place, pin, coords[0].x, coords[0].y, flip);
        let last_pos = Self::pos(
            place,
            pin,
            coords.last().unwrap().x,
            coords.last().unwrap().y,
            flip,
        );

        let last_first_segment = Line::new(last_pos, first_pos);

        if last_first_segment.length::<Euclidean>() >= MIN_FIRST_CHAIN_ELEMENT_LENGTH {
            maybe_first_chain_segment = Some((Line::new(last_pos, first_pos), 0));
        }

        for (index, coord_triple) in coords
            .iter()
            .circular_tuple_windows::<(_, _, _)>()
            .enumerate()
        {
            let curr_pos0 = Self::pos(place, pin, coord_triple.0.x, coord_triple.0.y, flip);
            let curr_pos1 = Self::pos(place, pin, coord_triple.1.x, coord_triple.1.y, flip);
            let curr_pos2 = Self::pos(place, pin, coord_triple.2.x, coord_triple.2.y, flip);
            let curr_segment01 = Line::new(curr_pos0, curr_pos1);
            let curr_segment12 = Line::new(curr_pos1, curr_pos2);

            if math::angle_between(curr_segment01.delta().into(), curr_segment12.delta().into())
                .abs()
                > 30.0_f64.to_radians()
                || curr_segment12.length::<Euclidean>() >= MIN_FIRST_CHAIN_ELEMENT_LENGTH
            {
                if let Some((first_chain_segment, first_chain_index)) = maybe_first_chain_segment {
                    if index - first_chain_index >= 3 {
                        let circle = math::fillet_circle(&first_chain_segment, &curr_segment12);

                        fillets.push(board.add_fixed_dot_infringably(
                            recorder,
                            FixedDotWeight(GeneralDotWeight {
                                circle,
                                layer,
                                maybe_net,
                            }),
                            None,
                        ));
                    }
                }

                maybe_first_chain_segment = Some((Line::new(curr_pos1, curr_pos2), index));
            }
        }

        fillets
    }

    fn pos(place: PointWithRotation, pin: PointWithRotation, x: f64, y: f64, flip: bool) -> Point {
        let pos = (Point::new(x, y) + pin.pos).rotate_around_point(pin.rot, pin.pos);
        (place.pos + flip.then_some(Point::new(-pos.x(), pos.y())).unwrap_or(pos))
            .rotate_around_point(place.rot, place.pos)
    }
}
