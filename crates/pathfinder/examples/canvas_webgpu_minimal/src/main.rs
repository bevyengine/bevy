// pathfinder/examples/canvas_webgpu_minimal/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_canvas::{Canvas, CanvasFontContext, Path2D};
use pathfinder_color::ColorF;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{vec2f, Vector2I};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_webgpu::WebGpuDevice;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

fn main() {
    let window_size = Vector2I::new(640, 480);

    let event_loop = EventLoop::new();
    let mut builder = winit::window::WindowBuilder::new()
        .with_title("Minimal example")
        .with_inner_size(winit::dpi::LogicalSize {
            width: window_size.x(),
            height: window_size.y(),
        });

    // Open a window.
    let window = builder.build(&event_loop).unwrap();

    let window_size = {
        let inner_size = window.inner_size();
        Vector2I::new(inner_size.width as i32, inner_size.height as i32)
    };

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(
        WebGpuDevice::new(window, window_size),
        &FilesystemResourceLoader::locate(),
        DestFramebuffer::full_window(window_size),
        RendererOptions {
            background_color: Some(ColorF::white()),
        },
    );

    // Make a canvas. We're going to draw a house.
    let mut canvas =
        Canvas::new(window_size.to_f32()).get_context_2d(CanvasFontContext::from_system_source());

    // Set line width.
    canvas.set_line_width(10.0);

    // Draw walls.
    canvas.stroke_rect(RectF::new(vec2f(75.0, 140.0), vec2f(150.0, 110.0)));

    // Draw door.
    canvas.fill_rect(RectF::new(vec2f(130.0, 190.0), vec2f(40.0, 60.0)));

    // Draw roof.
    let mut path = Path2D::new();
    path.move_to(vec2f(50.0, 140.0));
    path.line_to(vec2f(150.0, 60.0));
    path.line_to(vec2f(250.0, 140.0));
    path.close_path();
    canvas.stroke_path(path);

    // Render the canvas to screen.
    let scene = SceneProxy::from_scene(canvas.into_canvas().into_scene(), RayonExecutor);
    scene.build_and_render(&mut renderer, BuildOptions::default());
    renderer.device.end_frame();

    event_loop.run(|event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,

        _ => (),
    });
}
