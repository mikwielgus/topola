// SPDX-FileCopyrightText: 2026 Topola contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use specctra::{
    math::PointWithRotation,
    structure::{DsnFile, Shape},
};

use crate::{board::Board, layout::Joint, math::Vector2};

impl Board {
    pub fn from_specctra(dsn: DsnFile) -> Self {
        let mut board = Board::new(
            dsn.pcb
                .structure
                .boundary
                .coords()
                .into_owned()
                .into_iter()
                .map(|p| [p.x as i64, p.y as i64])
                .collect(),
        );

        // add pins from components
        for component in &dsn.pcb.placement.components {
            let image = dsn
                .pcb
                .library
                .images
                .iter()
                .find(|image| image.name == component.name)
                .unwrap();

            for place in &component.places {
                /*let place_side_is_front = place.side == "front";
                let get_layer = |board: &Board, name: &str| {
                    Self::layer(board, &dsn.pcb.structure.layers, name, place_side_is_front)
                };*/

                for pin in &image.pins {
                    let padstack = dsn.pcb.library.find_padstack_by_name(&pin.name).unwrap();

                    for shape in padstack.shapes.iter() {
                        match shape {
                            Shape::Circle(circle) => Self::place_circle(
                                &mut board,
                                place.point_with_rotation(),
                                pin.point_with_rotation(),
                                0,
                                (circle.diameter / 2.0) as u64,
                                false,
                            ),
                            _ => (),
                        }
                    }
                }
            }
        }

        board
    }

    pub fn place_circle(
        board: &mut Board,
        place: PointWithRotation,
        pin: PointWithRotation,
        layer: usize,
        radius: u64,
        flip: bool,
    ) {
        board.add_joint(Joint {
            position: Self::pos(place, pin, 0.0, 0.0, flip),
            layer,
            radius,
        });
    }

    /*pub fn place_rect(
        board: &mut Board,
        place: PointWithRotation,
        pin: PointWithRotation,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        layer: usize,
        flip: bool,
    ) {
        board.add_polygon(Polygon {
            vertices: [
                Self::pos(place, pin, x1, y1, flip),
                Self::pos(place, pin, x2, y1, flip),
                Self::pos(place, pin, x2, y2, flip),
                Self::pos(place, pin, x1, y2, flip),
            ],
            layer,
        })
    }*/

    fn pos(
        place: PointWithRotation,
        pin: PointWithRotation,
        x: f64,
        y: f64,
        flip: bool,
    ) -> [i64; 2] {
        let pos = (Vector2::new(x, y) + Vector2::new(pin.pos.x(), pin.pos.y()))
            .rotate_around_point_degrees(pin.rot, Vector2::new(pin.pos.x(), pin.pos.y()));
        let pos = (Vector2::new(place.pos.x(), place.pos.y())
            + flip.then_some(Vector2::new(-pos.x, pos.y)).unwrap_or(pos))
        .rotate_around_point_degrees(place.rot, Vector2::new(place.pos.x(), place.pos.y()));

        [pos.x as i64, pos.y as i64]
    }
}
