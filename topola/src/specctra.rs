// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use bidimap::BiBTreeMap;
use specctra::{
    math::PointWithRotation,
    structure::{DsnFile, Layer, Point, Shape},
};

use crate::{
    board::{Board, LayerDesc, LayerSide, LayerType},
    layout::LayerId,
    layout::compounds::{ComponentId, NetId, PinId},
    primitives::{JointSpec, Polygon, Segment, SegmentSpec},
    vector::Vector2,
};

impl Board {
    pub fn from_specctra(dsn: DsnFile) -> Self {
        let coordinate_scale = Self::coordinate_scale(&dsn);

        let top_outline_layer_id = LayerId::new(0);
        let pcb_layer_offset = 1;
        let bottom_outline_layer_id =
            LayerId::new(dsn.pcb.structure.layers.len() + pcb_layer_offset);

        let mut layer_descs =
            BiBTreeMap::from_iter(dsn.pcb.structure.layers.iter().enumerate().map(
                |(index, _layer)| {
                    let tier = if index == 0 {
                        LayerSide::Top
                    } else if index + 1 == dsn.pcb.structure.layers.len() {
                        LayerSide::Bottom
                    } else {
                        LayerSide::Inner
                    };
                    (
                        LayerId::new(index + pcb_layer_offset),
                        LayerDesc::new(LayerType::Copper, tier, index + pcb_layer_offset),
                    )
                },
            ));

        layer_descs.insert(
            top_outline_layer_id,
            LayerDesc::new(
                LayerType::Outline,
                LayerSide::Top,
                top_outline_layer_id.index(),
            ),
        );
        layer_descs.insert(
            bottom_outline_layer_id,
            LayerDesc::new(
                LayerType::Outline,
                LayerSide::Bottom,
                bottom_outline_layer_id.index(),
            ),
        );

        // assign IDs to all nets named in pcb.network
        let net_names = {
            let mut tmp: Vec<String> = dsn
                .pcb
                .network
                .classes
                .iter()
                .flat_map(|class| &class.nets)
                .chain(dsn.pcb.network.nets.iter().map(|net| &net.name))
                .cloned()
                .collect();
            tmp.sort_unstable();
            tmp.dedup();

            BiBTreeMap::from_iter(tmp.into_iter().enumerate().map(|(i, v)| (NetId::new(i), v)))
        };

        let mut board = Board::with_names(
            dsn.pcb
                .structure
                .boundary
                .coords()
                .into_owned()
                .into_iter()
                .skip(1)
                .rev()
                .map(|p| {
                    Vector2::new(
                        Self::scale_coord(p.x, coordinate_scale),
                        Self::scale_coord(p.y, coordinate_scale),
                    )
                })
                .collect(),
            layer_descs,
            net_names,
        );

        // Mapping of pin -> net prepared for adding pins.
        let pin_nets: BTreeMap<String, NetId> = dsn
            .pcb
            .network
            .nets
            .iter()
            .filter_map(|net_pin_assignments| {
                // Resolve the id so we don't work with strings.
                let net = board.net_id(&net_pin_assignments.name).unwrap();

                net_pin_assignments.pins.as_ref().map(|pins| {
                    // Take the list of pins
                    // and for each pin output (pin name, net id).
                    pins.names.iter().map(move |pinname| (pinname.clone(), net))
                })
            })
            // Flatten the nested iters into a single stream of tuples.
            .flatten()
            .collect();

        // Add pins from components.
        for component in &dsn.pcb.placement.components {
            let image = dsn
                .pcb
                .library
                .images
                .iter()
                .find(|image| image.name == component.name)
                .unwrap();

            for place in &component.places {
                let component_id = board.ensure_named_component(place.name.clone());

                let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board, name: &str| {
                    Self::layer(board, &dsn.pcb.structure.layers, name, place_side_is_front)
                };

                for outline in &image.outlines {
                    let outline_layer_id = if place_side_is_front {
                        top_outline_layer_id
                    } else {
                        bottom_outline_layer_id
                    };
                    Self::place_path(
                        &mut board,
                        place.point_with_rotation(),
                        PointWithRotation::default(),
                        &outline.path.coords,
                        outline.path.width,
                        outline_layer_id,
                        None,
                        Some(component_id),
                        None,
                        !place_side_is_front,
                        coordinate_scale,
                    );
                }

                for pin in &image.pins {
                    let pin_name = format!("{}-{}", place.name, pin.id);
                    let net_id = pin_nets.get(&pin_name).copied();

                    let pin_id = board.ensure_named_pin(pin_name.clone(), net_id);
                    let padstack = dsn.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => {
                                let layer = get_layer(&board, &circle.layer);
                                Self::place_circle(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    circle.diameter / 2.0,
                                    layer,
                                    net_id,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                    coordinate_scale,
                                )
                            }
                            Shape::Rect(rect) => {
                                let layer = get_layer(&board, &rect.layer);
                                Self::place_rect(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    rect.x1,
                                    rect.y1,
                                    rect.x2,
                                    rect.y2,
                                    layer,
                                    net_id,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                    coordinate_scale,
                                )
                            }
                            Shape::Path(path) => {
                                let layer = get_layer(&board, &path.layer);
                                Self::place_path(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &path.coords,
                                    path.width,
                                    layer,
                                    net_id,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                    coordinate_scale,
                                )
                            }
                            Shape::Polygon(polygon) => {
                                let layer = get_layer(&board, &polygon.layer);
                                Self::place_polygon(
                                    &mut board,
                                    place.point_with_rotation(),
                                    pin.point_with_rotation(),
                                    &polygon.coords,
                                    polygon.width,
                                    layer,
                                    net_id,
                                    Some(component_id),
                                    Some(pin_id),
                                    !place_side_is_front,
                                    coordinate_scale,
                                )
                            }
                        }
                    }
                }
            }
        }

        for via in &dsn.pcb.wiring.vias {
            let net = board.net_id(&via.net).unwrap();
            let padstack = dsn.pcb.library.find_padstack_by_name(&via.name).unwrap();

            let get_layer = |board: &Board, name: &str| {
                Self::layer(board, &dsn.pcb.structure.layers, name, true)
            };

            for shape in &padstack.shapes {
                match shape {
                    Shape::Circle(circle) => {
                        let layer = get_layer(&board, &circle.layer);
                        Self::place_circle(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            circle.diameter / 2.0,
                            layer,
                            Some(net),
                            None,
                            None,
                            false,
                            coordinate_scale,
                        )
                    }
                    Shape::Rect(rect) => {
                        let layer = get_layer(&board, &rect.layer);
                        Self::place_rect(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            rect.x1,
                            rect.y1,
                            rect.x2,
                            rect.y2,
                            layer,
                            Some(net),
                            None,
                            None,
                            false,
                            coordinate_scale,
                        )
                    }
                    Shape::Path(path) => {
                        let layer = get_layer(&board, &path.layer);
                        Self::place_path(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &path.coords,
                            path.width,
                            layer,
                            Some(net),
                            None,
                            None,
                            false,
                            coordinate_scale,
                        )
                    }
                    Shape::Polygon(polygon) => {
                        let layer = get_layer(&board, &polygon.layer);
                        Self::place_polygon(
                            &mut board,
                            PointWithRotation::from_xy(via.x, via.y),
                            PointWithRotation::default(),
                            &polygon.coords,
                            polygon.width,
                            layer,
                            Some(net),
                            None,
                            None,
                            false,
                            coordinate_scale,
                        )
                    }
                };
            }
        }

        for wire in dsn.pcb.wiring.wires.iter() {
            let layer = Self::layer(&board, &dsn.pcb.structure.layers, &wire.path.layer, true);
            let net = board.net_id(&wire.net).unwrap();

            Self::place_path(
                &mut board,
                PointWithRotation::default(),
                PointWithRotation::default(),
                &wire.path.coords,
                wire.path.width,
                layer,
                Some(net),
                None,
                None,
                false,
                coordinate_scale,
            );
        }

        board
    }

    fn place_circle(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        radius: f64,
        layer: LayerId,
        net: Option<NetId>,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
        coordinate_scale: f64,
    ) {
        board.insert_joint(JointSpec {
            position: Self::pos(place, pin_pos, 0.0, 0.0, flip, coordinate_scale),
            layer,
            net,
            component,
            pin,
            radius: Self::scale_size(radius, coordinate_scale),
        });
    }

    fn place_rect(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: LayerId,
        net: Option<NetId>,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
        coordinate_scale: f64,
    ) {
        board.insert_polygon(Polygon {
            vertices: vec![
                Self::pos(place, pin_pos, x1, y1, flip, coordinate_scale),
                Self::pos(place, pin_pos, x2, y1, flip, coordinate_scale),
                Self::pos(place, pin_pos, x2, y2, flip, coordinate_scale),
                Self::pos(place, pin_pos, x1, y2, flip, coordinate_scale),
            ],
            layer,
            net,
            component,
            pin,
        });
    }

    fn place_path(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        coords: &[Point],
        width: f64,
        layer: LayerId,
        net: Option<NetId>,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
        coordinate_scale: f64,
    ) {
        let mut prev_pos: Vector2<i64> = Self::pos(
            place,
            pin_pos,
            coords[0].x,
            coords[0].y,
            flip,
            coordinate_scale,
        );
        let mut prev_joint = board.insert_joint(JointSpec {
            position: prev_pos,
            layer,
            radius: Self::scale_size(width / 2.0, coordinate_scale),
            net,
            component,
            pin,
        });

        for coord in coords.iter().skip(1) {
            let pos = Self::pos(place, pin_pos, coord.x, coord.y, flip, coordinate_scale);

            if pos == prev_pos {
                continue;
            }

            let joint = board.insert_joint(JointSpec {
                position: pos,
                layer,
                radius: Self::scale_size(width / 2.0, coordinate_scale),
                net,
                component,
                pin,
            });

            let _ = board.insert_segment_raw(Segment {
                spec: SegmentSpec {
                    endjoints: [prev_joint, joint],
                    half_width: Self::scale_size(width / 2.0, coordinate_scale),
                    component,
                    pin,
                },
                endpoints: [prev_pos, pos],
                layer,
                net,
            });

            prev_pos = pos;
            prev_joint = joint;
        }
    }

    fn place_polygon(
        board: &mut Board,
        place: PointWithRotation,
        pin_pos: PointWithRotation,
        coords: &[Point],
        _width: f64,
        layer: LayerId,
        net: Option<NetId>,
        component: Option<ComponentId>,
        pin: Option<PinId>,
        flip: bool,
        coordinate_scale: f64,
    ) {
        let vertices: Vec<Vector2<i64>> = coords
            .iter()
            .map(|coord| Self::pos(place, pin_pos, coord.x, coord.y, flip, coordinate_scale))
            .collect();
        board.insert_polygon(Polygon {
            vertices,
            layer,
            net,
            component,
            pin,
        });
    }

    fn layer(_board: &Board, layers: &[Layer], name: &str, front: bool) -> LayerId {
        let pcb_layer_offset = 1;
        let image_layer = LayerId::new(
            layers.iter().position(|layer| layer.name == name).unwrap() + pcb_layer_offset,
        );
        let image_layer_index = image_layer.index() - pcb_layer_offset;

        if front {
            image_layer
        } else {
            LayerId::new(layers.len() - image_layer_index)
        }
    }

    fn pos(
        place: PointWithRotation,
        pin: PointWithRotation,
        x: f64,
        y: f64,
        flip: bool,
        coordinate_scale: f64,
    ) -> Vector2<i64> {
        let pos = (Vector2::new(x, y) + Vector2::new(pin.pos.x(), pin.pos.y()))
            .rotate_around_point_degrees(pin.rot, Vector2::new(pin.pos.x(), pin.pos.y()));
        let pos = (Vector2::new(place.pos.x(), place.pos.y())
            + flip.then_some(Vector2::new(-pos.x, pos.y)).unwrap_or(pos))
        .rotate_around_point_degrees(place.rot, Vector2::new(place.pos.x(), place.pos.y()));

        Vector2::new(
            Self::scale_coord(pos.x, coordinate_scale),
            Self::scale_coord(pos.y, coordinate_scale),
        )
    }

    fn coordinate_scale(dsn: &DsnFile) -> f64 {
        let unit = dsn
            .pcb
            .unit
            .as_deref()
            .unwrap_or(&dsn.pcb.resolution.unit)
            .to_ascii_lowercase();

        match unit.as_str() {
            "um" | "µm" => 1.0,
            "mm" => 1000.0,
            "cm" => 10_000.0,
            "m" => 1_000_000.0,
            "mil" => 25.4,
            "in" | "inch" => 25_400.0,
            "nm" => 0.001,
            _ => 1.0,
        }
    }

    fn scale_coord(value: f64, coordinate_scale: f64) -> i64 {
        (value * coordinate_scale).round() as i64
    }

    fn scale_size(value: f64, coordinate_scale: f64) -> u64 {
        ((value * coordinate_scale).round()).max(0.0) as u64
    }
}
