#![feature(try_blocks)]
extern crate sdl2;

macro_rules! dbg_dot {
    ($graph:expr) => {
        use petgraph::dot::Dot;
        println!("{:?}", Dot::new(&$graph));
    };
}

mod astar;
mod connectivity;
mod draw;
mod geometry;
mod graph;
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
mod triangulation;

use draw::DrawException;
use geo::point;
use geometry::{FixedDotIndex, FixedSegWeight, GeometryIndex, LooseDotIndex, MakePrimitive};
use layout::{Infringement, Layout, LayoutException};
use mesh::{Mesh, MeshEdgeReference, VertexIndex};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use primitive::MakeShape;
use router::RouterObserver;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{GLProfile, Window};
use sdl2::EventPump;
use shape::{Shape, ShapeTrait};

use pathfinder_canvas::{ArcDirection, CanvasRenderingContext2D, ColorU, FillRule};
use pathfinder_canvas::{Canvas, CanvasFontContext, Path2D};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use std::time::Duration;
use tracer::{Trace, Tracer};

use crate::geometry::FixedDotWeight;
use crate::math::Circle;
use crate::router::Router;

// Clunky enum to work around borrow checker.
enum RouterOrLayout<'a> {
    Router(&'a mut Router),
    Layout(&'a Layout),
}

struct EmptyRouterObserver;

impl RouterObserver for EmptyRouterObserver {
    fn on_rework(&mut self, _tracer: &Tracer, _trace: &Trace) {}
    fn before_probe(&mut self, _tracer: &Tracer, _trace: &Trace, _edge: MeshEdgeReference) {}
    fn on_probe(
        &mut self,
        _tracer: &Tracer,
        _trace: &Trace,
        _edge: MeshEdgeReference,
        _result: Result<(), DrawException>,
    ) {
    }
    fn on_estimate(&mut self, _tracer: &Tracer, _vertex: VertexIndex) {}
}

struct DebugRouterObserver<'a> {
    event_pump: &'a mut sdl2::EventPump,
    window: &'a Window,
    renderer: &'a mut Renderer<GLDevice>,
    font_context: &'a CanvasFontContext,
}

impl<'a> DebugRouterObserver<'a> {
    pub fn new(
        event_pump: &'a mut sdl2::EventPump,
        window: &'a Window,
        renderer: &'a mut Renderer<GLDevice>,
        font_context: &'a CanvasFontContext,
    ) -> Self {
        Self {
            event_pump,
            window,
            renderer,
            font_context,
        }
    }
}

impl<'a> RouterObserver for DebugRouterObserver<'a> {
    fn on_rework(&mut self, tracer: &Tracer, trace: &Trace) {
        render_times(
            self.event_pump,
            self.window,
            self.renderer,
            self.font_context,
            RouterOrLayout::Layout(tracer.layout),
            None,
            None,
            Some(tracer.mesh.clone()),
            &trace.path,
            &[],
            &[],
            40,
        );
    }

    fn before_probe(&mut self, tracer: &Tracer, trace: &Trace, edge: MeshEdgeReference) {
        let mut path = trace.path.clone();
        path.push(edge.target());
        render_times(
            self.event_pump,
            self.window,
            self.renderer,
            self.font_context,
            RouterOrLayout::Layout(tracer.layout),
            None,
            None,
            Some(tracer.mesh.clone()),
            &path,
            &[],
            &[],
            10,
        );
    }

    fn on_probe(
        &mut self,
        tracer: &Tracer,
        trace: &Trace,
        _edge: MeshEdgeReference,
        result: Result<(), DrawException>,
    ) {
        let (ghosts, highlighteds, delay) = match result {
            Err(DrawException::CannotWrapAround(
                ..,
                LayoutException::Infringement(Infringement(shape1, infringee1)),
                LayoutException::Infringement(Infringement(shape2, infringee2)),
            )) => (vec![shape1, shape2], vec![infringee1, infringee2], 30),
            _ => (vec![], vec![], 10),
        };

        render_times(
            self.event_pump,
            self.window,
            self.renderer,
            self.font_context,
            RouterOrLayout::Layout(tracer.layout),
            None,
            None,
            Some(tracer.mesh.clone()),
            &trace.path,
            &ghosts,
            &highlighteds,
            delay,
        );
    }

    fn on_estimate(&mut self, _tracer: &Tracer, _vertex: VertexIndex) {}
}

fn main() -> Result<(), anyhow::Error> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(4, 0);

    let window = video_subsystem
        .window("Topola demo", 800, 600)
        .opengl()
        .position_centered()
        .build()
        .unwrap();

    let _context = window.gl_create_context().unwrap();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    // doing this later (after pathfinder assumes control of the context) would be a bad idea
    // but for the early clear it's simpler than passing a blank canvas to pathfinder
    unsafe {
        gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
    window.gl_swap_window();

    // XXX: not sure if this automatically fallbacks if we get a GL3 context
    // or if we have to detect it and/or retry
    let device = GLDevice::new(GLVersion::GL4, 0);

    let mode = RendererMode::default_for_device(&device);
    let options = RendererOptions {
        dest: DestFramebuffer::full_window(vec2i(800, 600)),
        background_color: Some(ColorU::black().to_f32()),
        //show_debug_ui: true,
        ..RendererOptions::default()
    };
    let resource_loader = EmbeddedResourceLoader::new();
    let mut renderer = Renderer::new(device, &resource_loader, mode, options);
    let font_context = CanvasFontContext::from_system_source();

    // TODO: make a type like this wrapping the details of pathfinder
    // so we don't pass so many arguments to render_times() and through the debug observer
    //let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let _i = 0;
    let mut router = Router::new();

    let component1_1 = router.layout.add_component(1);
    let component1_2 = router.layout.add_component(1);
    let component2 = router.layout.add_component(2);
    let component3_1 = router.layout.add_component(3);
    let component3_2 = router.layout.add_component(3);
    let component4_1 = router.layout.add_component(4);
    let component4_2 = router.layout.add_component(4);

    let dot_start = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component1_1,
            circle: Circle {
                pos: (100.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot_start2 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component3_1,
            circle: Circle {
                pos: (100.5, 500.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot_start3 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component4_1,
            circle: Circle {
                pos: (160.5, 430.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot_end = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component1_2,
            circle: Circle {
                pos: (470.5, 350.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot_end2 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component3_2,
            circle: Circle {
                pos: (500.5, 150.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot_end3 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component4_2,
            circle: Circle {
                pos: (350.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot1_1 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (200.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot2_1 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (200.5, 500.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot1_1,
        dot2_1,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let dot2_2 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (600.5, 500.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot2_1,
        dot2_2,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let dot3 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (400.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let dot4 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (400.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot3,
        dot4,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let dot5 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (530.5, 400.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot4,
        dot5,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let dot1_2 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (600.5, 200.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot3,
        dot1_2,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let _ = router.layout.add_fixed_seg(
        dot1_2,
        dot2_2,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    let dot6 = router
        .layout
        .add_fixed_dot(FixedDotWeight {
            component: component2,
            circle: Circle {
                pos: (530.5, 300.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_fixed_seg(
        dot5,
        dot6,
        FixedSegWeight {
            component: component2,
            width: 16.0,
        },
    );

    /*let dot7 = router
        .layout
        .add_dot(FixedDotWeight {
            net: 2,
            circle: Circle {
                pos: (400.5, 440.5).into(),
                r: 8.0,
            },
        })
        .unwrap();

    let _ = router.layout.add_seg(
        dot4,
        dot7,
        FixedSegWeight {
            net: 20,
            width: 16.0,
        },
    );*/

    /*render_times(
        &mut event_pump,
        &mut canvas,
        RouterOrLayout::Layout(&router.layout),
        None,
        None,
        None,
        &[],
        -1,
    );

    render_times(
        &mut event_pump,
        &mut canvas,
        RouterOrLayout::Router(&mut router),
        Some(dot_start),
        Some(dot_end),
        None,
        &[],
        -1,
    );*/

    let _ = router.enroute(
        dot_start,
        dot_end,
        &mut EmptyRouterObserver,
        //&mut DebugRouterObserver::new(&mut event_pump, &window, &mut renderer, &font_context),
    )?;

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        RouterOrLayout::Layout(&router.layout),
        None,
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    /*render_times(
        &mut event_pump,
        &mut canvas,
        RouterOrLayout::Router(&mut router),
        Some(dot_start2),
        Some(dot_end),
        None,
        &[],
        -1,
    );*/

    let _ = router.enroute(
        dot_start2,
        dot_end2,
        &mut EmptyRouterObserver,
        //&mut DebugRouterObserver::new(&mut event_pump, &window, &mut renderer, &font_context),
    )?;

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        RouterOrLayout::Layout(&router.layout),
        None,
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    let _ = router.enroute(
        dot_start3,
        dot_end3,
        &mut DebugRouterObserver::new(&mut event_pump, &window, &mut renderer, &font_context),
    )?;

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        RouterOrLayout::Layout(&router.layout),
        None,
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    Ok(())
}

fn render_times(
    event_pump: &mut EventPump,
    window: &Window,
    renderer: &mut Renderer<GLDevice>,
    font_context: &CanvasFontContext,
    mut router_or_layout: RouterOrLayout,
    from: Option<FixedDotIndex>,
    follower: Option<LooseDotIndex>,
    mut mesh: Option<Mesh>,
    path: &[VertexIndex],
    ghosts: &[Shape],
    highlighteds: &[GeometryIndex],
    times: i64,
) {
    let mut i = 0;

    'running: loop {
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

        renderer.options_mut().background_color = Some(ColorU::new(0, 10, 35, 255).to_f32());

        let window_size = window.size();
        let mut canvas = Canvas::new(vec2f(window_size.0 as f32, window_size.1 as f32))
            .get_context_2d(font_context.clone());

        let layout = match router_or_layout {
            RouterOrLayout::Router(ref mut router) => {
                if let Some(follower) = follower {
                    let state = event_pump.mouse_state();

                    if let Some(from) = from {
                        mesh = router
                            .reroute(
                                from,
                                point! {x: state.x() as f64, y: state.y() as f64},
                                &mut DebugRouterObserver::new(
                                    event_pump,
                                    window,
                                    renderer,
                                    font_context,
                                ),
                            )
                            .ok();
                    } else {
                        let _ = router
                            .layout
                            .move_dot(follower, (state.x() as f64, state.y() as f64).into());
                    }
                }

                &router.layout
            }
            RouterOrLayout::Layout(layout) => layout,
        };

        //let result = panic::catch_unwind(|| {
        for node in layout.nodes() {
            let color = if highlighteds.contains(&node) {
                ColorU::new(255, 100, 100, 255)
            } else {
                ColorU::new(200, 52, 52, 255)
            };

            let shape = node.primitive(layout).shape();
            render_shape(&mut canvas, &shape, color);
        }

        for ghost in ghosts {
            render_shape(&mut canvas, &ghost, ColorU::new(75, 75, 150, 255));
        }

        if let Some(ref mesh) = mesh {
            for edge in mesh.edge_references() {
                let start_point = edge.source().primitive(layout).shape().center();
                let end_point = edge.target().primitive(layout).shape().center();

                let color = 'blk: {
                    if let (Some(source_pos), Some(target_pos)) = (
                        path.iter().position(|node| *node == edge.source()),
                        path.iter().position(|node| *node == edge.target()),
                    ) {
                        if target_pos == source_pos + 1 {
                            break 'blk ColorU::new(250, 250, 0, 255);
                        }
                    }

                    ColorU::new(125, 125, 125, 255)
                };

                let mut path = Path2D::new();
                path.move_to(vec2f(start_point.x() as f32, start_point.y() as f32));
                path.line_to(vec2f(end_point.x() as f32, end_point.y() as f32));
                canvas.set_stroke_style(color);
                canvas.set_line_width(1.0);
                canvas.stroke_path(path);
            }
        }
        //});

        let mut scene = SceneProxy::from_scene(
            canvas.into_canvas().into_scene(),
            renderer.mode().level,
            RayonExecutor,
        );
        scene.build_and_render(renderer, BuildOptions::default());
        window.gl_swap_window();

        i += 1;
        if times != -1 && i >= times {
            return;
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

fn render_shape(canvas: &mut CanvasRenderingContext2D, shape: &Shape, color: ColorU) {
    canvas.set_stroke_style(color);
    canvas.set_fill_style(color);

    match shape {
        Shape::Dot(dot) => {
            let mut path = Path2D::new();
            path.ellipse(
                vec2f(dot.c.pos.x() as f32, dot.c.pos.y() as f32),
                dot.c.r as f32,
                0.0,
                0.0,
                std::f32::consts::TAU,
            );
            canvas.fill_path(path, FillRule::Winding);
        }
        Shape::Seg(seg) => {
            let mut path = Path2D::new();
            path.move_to(vec2f(seg.from.x() as f32, seg.from.y() as f32));
            path.line_to(vec2f(seg.to.x() as f32, seg.to.y() as f32));
            canvas.set_line_width(seg.width as f32);
            canvas.stroke_path(path);
        }
        Shape::Bend(bend) => {
            let delta1 = bend.from - bend.c.pos;
            let delta2 = bend.to - bend.c.pos;

            let angle1 = delta1.y().atan2(delta1.x());
            let angle2 = delta2.y().atan2(delta2.x());

            let mut path = Path2D::new();
            path.arc(
                vec2f(bend.c.pos.x() as f32, bend.c.pos.y() as f32),
                bend.circle().r as f32,
                angle1 as f32,
                angle2 as f32,
                ArcDirection::CW,
            );
            canvas.set_line_width(bend.width as f32);
            canvas.stroke_path(path);
        }
    }
    let envelope = ShapeTrait::envelope(shape);
    // XXX: points represented as arrays can't be conveniently converted to vector types
    let topleft = vec2f(envelope.lower()[0] as f32, envelope.lower()[1] as f32);
    let bottomright = vec2f(envelope.upper()[0] as f32, envelope.upper()[1] as f32);
    canvas.set_line_width(1.0);
    canvas.set_stroke_style(ColorU::new(100, 100, 100, 255));
    canvas.stroke_rect(RectF::new(topleft, bottomright - topleft));
}
