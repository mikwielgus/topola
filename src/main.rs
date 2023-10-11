extern crate sdl2;

macro_rules! dbg_dot {
    ($graph:expr) => {
        use petgraph::dot::Dot;
        println!("{:?}", Dot::new(&$graph));
    };
}

#[macro_use]
mod graph;
mod astar;
mod band;
mod bow;
mod draw;
mod guide;
mod layout;
mod math;
mod mesh;
mod primitive;
mod router;
mod rules;
mod segbend;
mod shape;
mod tracer;

use geo::point;
use graph::{DotIndex, SegWeight};
use layout::Layout;
use mesh::{Mesh, MeshEdgeReference, VertexIndex};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use router::RouterObserver;
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;
use shape::Shape;
use std::panic;
use std::time::Duration;
use tracer::Tracer;

use crate::graph::DotWeight;
use crate::math::Circle;
use crate::router::Router;

// Clunky enum to work around borrow checker.
enum RouterOrLayout<'a> {
    Router(&'a mut Router),
    Layout(&'a Layout),
}

struct DebugRouterObserver<'a> {
    event_pump: &'a mut sdl2::EventPump,
    canvas: &'a mut sdl2::render::Canvas<Window>,
}

impl<'a> DebugRouterObserver<'a> {
    pub fn new(
        event_pump: &'a mut sdl2::EventPump,
        canvas: &'a mut sdl2::render::Canvas<Window>,
    ) -> Self {
        Self { event_pump, canvas }
    }
}

impl<'a> RouterObserver for DebugRouterObserver<'a> {
    fn on_rework(&mut self, tracer: &Tracer, path: &[VertexIndex]) {
        render_times(
            self.event_pump,
            self.canvas,
            RouterOrLayout::Layout(tracer.layout),
            None,
            None,
            Some(tracer.mesh.clone()),
            path,
            10,
        );
    }

    fn on_probe(&mut self, _tracer: &Tracer, _edge: MeshEdgeReference) {}
    fn on_estimate(&mut self, _tracer: &Tracer, _vertex: VertexIndex) {}
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Topola demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let _i = 0;
    let mut router = Router::new();

    let dot1 = router
        .layout
        .add_dot(DotWeight {
            net: 1,
            circle: Circle {
                pos: (100.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot2 = router
        .layout
        .add_dot(DotWeight {
            net: 1,
            circle: Circle {
                pos: (500.5, 430.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot1_1 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (200.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot2_1 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (200.5, 500.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_seg(
        dot1_1,
        dot2_1,
        SegWeight {
            net: 20,
            width: 16.0,
        },
    );

    let dot2_2 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (600.5, 500.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_seg(
        dot2_1,
        dot2_2,
        SegWeight {
            net: 20,
            width: 16.0,
        },
    );

    let dot3 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (400.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot4 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (400.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_seg(
        dot3,
        dot4,
        SegWeight {
            net: 20,
            width: 16.0,
        },
    );

    let dot1_2 = router
        .layout
        .add_dot(DotWeight {
            net: 2,
            circle: Circle {
                pos: (600.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_seg(
        dot3,
        dot1_2,
        SegWeight {
            net: 20,
            width: 16.0,
        },
    );

    render_times(
        &mut event_pump,
        &mut canvas,
        RouterOrLayout::Router(&mut router),
        Some(dot1),
        Some(dot2),
        None,
        &[],
        -1,
    );
    render_times(
        &mut event_pump,
        &mut canvas,
        RouterOrLayout::Layout(&router.layout),
        None,
        None,
        None,
        &[],
        -1,
    );
}

fn render_times(
    event_pump: &mut EventPump,
    canvas: &mut Canvas<Window>,
    mut router_or_layout: RouterOrLayout,
    from: Option<DotIndex>,
    follower: Option<DotIndex>,
    mut mesh: Option<Mesh>,
    path: &[VertexIndex],
    times: i64,
) {
    let mut i = 0;

    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 10, 35));
        canvas.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let layout = match router_or_layout {
            RouterOrLayout::Router(ref mut router) => {
                if let Some(follower) = follower {
                    let state = event_pump.mouse_state();

                    /*let _ = router.layout.move_dot(
                        *follower.as_dot().unwrap(),
                        (state.x() as f64, state.y() as f64).into(),
                    );*/

                    if let Some(from) = from {
                        mesh = router
                            .reroute(
                                from,
                                point! {x: state.x() as f64, y: state.y() as f64},
                                &mut DebugRouterObserver::new(event_pump, canvas),
                            )
                            .ok();
                    }
                }

                &router.layout
            }
            RouterOrLayout::Layout(layout) => layout,
        };

        //let result = panic::catch_unwind(|| {
        for shape in layout.shapes() {
            match shape {
                Shape::Dot(dot) => {
                    let _ = canvas.filled_circle(
                        dot.c.pos.x() as i16,
                        dot.c.pos.y() as i16,
                        dot.c.r as i16,
                        Color::RGB(200, 52, 52),
                    );
                }
                Shape::Seg(seg) => {
                    let _ = canvas.thick_line(
                        seg.from.x() as i16,
                        seg.from.y() as i16,
                        seg.to.x() as i16,
                        seg.to.y() as i16,
                        seg.width as u8,
                        Color::RGB(200, 52, 52),
                    );
                }
                Shape::Bend(bend) => {
                    let delta1 = bend.from - bend.c.pos;
                    let delta2 = bend.to - bend.c.pos;

                    let angle1 = delta1.y().atan2(delta1.x());
                    let angle2 = delta2.y().atan2(delta2.x());

                    for d in -2..3 {
                        let _ = canvas.arc(
                            //around_circle.pos.x() as i16,
                            //around_circle.pos.y() as i16,
                            bend.c.pos.x() as i16,
                            bend.c.pos.y() as i16,
                            //(shape.around_weight.unwrap().circle.r + 10.0 + (d as f64)) as i16,
                            (bend.circle().r + (d as f64)) as i16,
                            angle1.to_degrees() as i16,
                            angle2.to_degrees() as i16,
                            Color::RGB(200, 52, 52),
                        );
                    }
                }
            }
            /*let envelope = shape.envelope();
            let _ = canvas.rectangle(
                envelope.lower()[0] as i16,
                envelope.lower()[1] as i16,
                envelope.upper()[0] as i16,
                envelope.upper()[1] as i16,
                Color::RGB(100, 100, 100),
            );*/
        }

        if let Some(ref mesh) = mesh {
            for edge in mesh.edge_references() {
                let endpoints = (mesh.position(edge.source()), mesh.position(edge.target()));

                let color = if path.contains(&edge.source()) && path.contains(&edge.target()) {
                    Color::RGB(250, 250, 0)
                } else {
                    Color::RGB(125, 125, 125)
                };

                let _ = canvas.line(
                    endpoints.0.x() as i16,
                    endpoints.0.y() as i16,
                    endpoints.1.x() as i16,
                    endpoints.1.y() as i16,
                    color,
                );
            }
        }
        //});

        canvas.present();

        i += 1;
        if times != -1 && i >= times {
            return;
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
