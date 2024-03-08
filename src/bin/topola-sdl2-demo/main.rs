extern crate sdl2;

mod painter;

macro_rules! dbg_dot {
    ($graph:expr) => {
        use petgraph::dot::Dot;
        println!("{:?}", Dot::new(&$graph));
    };
}

use geo::point;
use painter::Painter;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use topola::board::connectivity::BandIndex;
use topola::board::Board;
use topola::draw::DrawException;
use topola::dsn::design::DsnDesign;
use topola::geometry::shape::{Shape, ShapeTrait};
use topola::layout::dot::FixedDotWeight;
use topola::layout::graph::{GeometryIndex, MakePrimitive};
use topola::layout::primitive::MakeShape;
use topola::layout::rules::{Conditions, RulesTrait};
use topola::layout::seg::FixedSegWeight;
use topola::layout::{Infringement, Layout, LayoutException};
use topola::mesh::{Mesh, MeshEdgeReference, VertexIndex};
use topola::router::RouterObserverTrait;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{GLProfile, Window};
use sdl2::EventPump;

use pathfinder_canvas::{Canvas, CanvasFontContext, ColorU};
use pathfinder_geometry::vector::{vec2f, vec2i, Vector2F};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererMode, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::embedded::EmbeddedResourceLoader;

use std::collections::HashMap;
use std::time::Duration;
use topola::tracer::{Trace, Tracer};

use topola::math::Circle;
use topola::router::Router;

struct SimpleRules {
    net_clearances: HashMap<(i64, i64), f64>,
}

impl RulesTrait for SimpleRules {
    fn clearance(&self, conditions1: &Conditions, conditions2: &Conditions) -> f64 {
        *self
            .net_clearances
            .get(&(conditions1.net, conditions2.net))
            .unwrap_or(&10.0)
    }

    fn largest_clearance(&self, net: i64) -> f64 {
        let mut highest_clearance = 0.0;

        for ((net1, net2), clearance) in self.net_clearances.iter() {
            if *net1 == net || *net2 == net {
                highest_clearance = *clearance;
            }
        }

        highest_clearance
    }
}

// Clunky enum to work around borrow checker.
enum RouterOrLayout<'a, R: RulesTrait> {
    Router(&'a mut Router<R>),
    Layout(&'a Layout<R>),
}

struct EmptyRouterObserver;

impl<R: RulesTrait> RouterObserverTrait<R> for EmptyRouterObserver {
    fn on_rework(&mut self, _tracer: &Tracer<R>, _trace: &Trace) {}
    fn before_probe(&mut self, _tracer: &Tracer<R>, _trace: &Trace, _edge: MeshEdgeReference) {}
    fn on_probe(
        &mut self,
        _tracer: &Tracer<R>,
        _trace: &Trace,
        _edge: MeshEdgeReference,
        _result: Result<(), DrawException>,
    ) {
    }
    fn on_estimate(&mut self, _tracer: &Tracer<R>, _vertex: VertexIndex) {}
}

struct DebugRouterObserver<'a> {
    event_pump: &'a mut sdl2::EventPump,
    window: &'a Window,
    renderer: &'a mut Renderer<GLDevice>,
    font_context: &'a CanvasFontContext,
    view: &'a mut View,
}

impl<'a> DebugRouterObserver<'a> {
    pub fn new(
        event_pump: &'a mut sdl2::EventPump,
        window: &'a Window,
        renderer: &'a mut Renderer<GLDevice>,
        font_context: &'a CanvasFontContext,
        view: &'a mut View,
    ) -> Self {
        Self {
            event_pump,
            window,
            renderer,
            font_context,
            view,
        }
    }
}

impl<'a, R: RulesTrait> RouterObserverTrait<R> for DebugRouterObserver<'a> {
    fn on_rework(&mut self, tracer: &Tracer<R>, trace: &Trace) {
        render_times(
            self.event_pump,
            self.window,
            self.renderer,
            self.font_context,
            self.view,
            RouterOrLayout::Layout(tracer.board.layout()),
            None,
            Some(tracer.mesh.clone()),
            &trace.path,
            &[],
            &[],
            40,
        );
    }

    fn before_probe(&mut self, tracer: &Tracer<R>, trace: &Trace, edge: MeshEdgeReference) {
        let mut path = trace.path.clone();
        path.push(edge.target());
        render_times(
            self.event_pump,
            self.window,
            self.renderer,
            self.font_context,
            self.view,
            RouterOrLayout::Layout(tracer.board.layout()),
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
        tracer: &Tracer<R>,
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
            self.view,
            RouterOrLayout::Layout(tracer.board.layout()),
            None,
            Some(tracer.mesh.clone()),
            &trace.path,
            &ghosts,
            &highlighteds,
            delay,
        );
    }

    fn on_estimate(&mut self, _tracer: &Tracer<R>, _vertex: VertexIndex) {}
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
    /*let mut router = Router::new(Layout::new(SimpleRules {
        net_clearances: HashMap::from([
            ((1, 2), 8.0),
            ((2, 1), 8.0),
            ((2, 3), 3.0),
            ((3, 2), 3.0),
            ((3, 4), 15.0),
            ((4, 3), 15.0),
        ]),
    }));*/

    let design = DsnDesign::load_from_file("tests/data/prerouted_lm317_breakout/prerouted_lm317_breakout.dsn")?;
    //let design = DsnDesign::load_from_file("tests/data/test/test.dsn")?;
    //dbg!(&design);
    let layout = design.make_layout();
    let board = Board::new(layout);
    let mut router = Router::new(board);

    let mut view = View { pan: vec2f(-400.0, -300.0), zoom: 0.5 };

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        &mut view,
        RouterOrLayout::Layout(router.board.layout()),
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    // these are both on net 1 in the test file
    /*let _ = router.route_band(
        dot_indices[1],
        dot_indices[2],
        3.0,
        //&mut EmptyRouterObserver,
        &mut DebugRouterObserver::new(&mut event_pump, &window, &mut renderer, &font_context),
    )?;*/

    render_times(
        &mut event_pump,
        &window,
        &mut renderer,
        &font_context,
        &mut view,
        RouterOrLayout::Layout(router.board.layout()),
        None,
        None,
        &[],
        &[],
        &[],
        -1,
    );

    Ok(())
}

struct View {
    pan: Vector2F,
    zoom: f32,
}

fn render_times(
    event_pump: &mut EventPump,
    window: &Window,
    renderer: &mut Renderer<GLDevice>,
    font_context: &CanvasFontContext,
    view: &mut View,
    mut router_or_layout: RouterOrLayout<impl RulesTrait>,
    maybe_band: Option<BandIndex>,
    mut maybe_mesh: Option<Mesh>,
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
                Event::MouseWheel { y, .. } => {
                    view.zoom *= f32::powf(1.4, y as f32);
                },
                Event::MouseMotion { xrel, yrel, mousestate, .. } => {
                    if mousestate.left() {
                        view.pan += vec2f(xrel as f32, yrel as f32) / view.zoom;
                    }
                },
                _ => {}
            }
        }

        renderer.options_mut().background_color = Some(ColorU::new(0, 10, 35, 255).to_f32());

        let window_size = window.size();
        let mut canvas = Canvas::new(vec2f(window_size.0 as f32, window_size.1 as f32))
            .get_context_2d(font_context.clone());

        let center = vec2f(400.0, 300.0);
        canvas.translate(center);
        canvas.scale(vec2f(view.zoom, view.zoom));
        canvas.translate(-center + view.pan);

        let mut painter = Painter::new(&mut canvas);

        let layout = match router_or_layout {
            RouterOrLayout::Router(ref mut router) => {
                let state = event_pump.mouse_state();

                if let Some(band) = maybe_band {
                    router
                        .reroute_band(
                            band,
                            point! {x: state.x() as f64, y: state.y() as f64},
                            3.0,
                            &mut DebugRouterObserver::new(
                                event_pump,
                                window,
                                renderer,
                                font_context,
                                view,
                            ),
                        )
                        .ok();
                    maybe_mesh = None;
                }

                router.board.layout()
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
            painter.paint_shape(&shape, color);
        }

        for ghost in ghosts {
            painter.paint_shape(&ghost, ColorU::new(75, 75, 150, 255));
        }

        if let Some(ref mesh) = maybe_mesh {
            for edge in mesh.edge_references() {
                let to = edge.source().primitive(layout).shape().center();
                let from = edge.target().primitive(layout).shape().center();

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

                painter.paint_edge(from, to, color);
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
