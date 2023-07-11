extern crate sdl2;

mod layout;
mod rules;
mod mesh;
mod primitive;
mod weight;
mod math;

use std::mem::swap;
use std::time::Duration;
use geo::geometry::Point;
use mesh::Index;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;

use crate::layout::Layout;
use crate::primitive::Primitive;
use crate::weight::{Weight, DotWeight};
use crate::math::Circle;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    let mut layout = Layout::new();
    
    let index = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (150.5, 80.5).into(), r: 8.0}});
    //layout.route_seg(index, Point {x: 400.5, y: 350.5}, 6.0);

    let index2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (180.5, 150.5).into(), r: 8.0}});
    let barrier1 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (90.5, 150.5).into(), r: 8.0}});
    layout.add_seg(index2, barrier1, 16.0);

    let index3 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (130.5, 250.5).into(), r: 8.0}});
    let barrier2 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (190.5, 250.5).into(), r: 8.0}});
    layout.add_seg(index3, barrier2, 16.0);

    let index4 = layout.route_around(index, index2, true, 5.0);
    let index5 = layout.route_around(index4, index3, false, 5.0);

    let index6 = layout.add_dot(DotWeight {net: 0, circle: Circle {pos: (140.5, 300.5).into(), r: 8.0}});
    let index7 = layout.route_to(index5, index6, 5.0);

    'running: loop {
        i = (i + 1) % 255;

        canvas.set_draw_color(Color::RGB(0, 10, 35));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                }
                _ => {}
            }
        }

        for primitive in layout.primitives() {
            match primitive.weight {
                Weight::Dot(dot) => {
                    let _ = canvas.filled_circle(dot.circle.pos.x() as i16,
                                                 dot.circle.pos.y() as i16,
                                                 dot.circle.r as i16,
                                                 Color::RGB(200, 52, 52));
                },
                Weight::Seg(seg) => {
                    let dot_neighbor_weights = primitive.dot_neighbor_weights;
                    let _ = canvas.thick_line(dot_neighbor_weights[0].circle.pos.x() as i16,
                                              dot_neighbor_weights[0].circle.pos.y() as i16,
                                              dot_neighbor_weights[1].circle.pos.x() as i16,
                                              dot_neighbor_weights[1].circle.pos.y() as i16,
                                              seg.width as u8,
                                              Color::RGB(200, 52, 52));
                },
                Weight::Bend(bend) => {
                    let dot_neighbor_weights = primitive.dot_neighbor_weights;
                    let around_circle = primitive.around_weight.unwrap().circle;

                    let delta1 = dot_neighbor_weights[0].circle.pos - around_circle.pos;
                    let delta2 = dot_neighbor_weights[1].circle.pos - around_circle.pos;

                    let mut angle1 = delta1.y().atan2(delta1.x());
                    let mut angle2 = delta2.y().atan2(delta2.x());

                    if !primitive.weight.as_bend().unwrap().cw {
                        swap(&mut angle1, &mut angle2);
                    }

                    for d in -3..3 {
                        let _ = canvas.arc(
                            around_circle.pos.x() as i16,
                            around_circle.pos.y() as i16,
                            (primitive.around_weight.unwrap().circle.r + 10.0 + (d as f64)) as i16,
                            angle1.to_degrees() as i16,
                            angle2.to_degrees() as i16,
                            Color::RGB(200, 52, 52));
                    }

                },
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
