// pathfinder/examples/canvas_glutin_minimal/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Demonstrates how to use the Pathfinder canvas API with `glutin`.

use glutin::dpi::PhysicalSize;
use glutin::{ContextBuilder, GlProfile, GlRequest};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glutin::window::WindowBuilder;
use pathfinder_canvas::{Canvas, CanvasFontContext, Path2D};
use pathfinder_color::ColorF;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::options::BuildOptions;

fn main() {
    // Calculate the right logical size of the window.
    let event_loop = EventLoop::new();
    let window_size = vec2i(640, 480);
    let physical_window_size = PhysicalSize::new(window_size.x() as f64, window_size.y() as f64);

    // Open a window.
    let window_builder = WindowBuilder::new().with_title("Minimal example")
                                             .with_inner_size(physical_window_size);

    // Create an OpenGL 3.x context for Pathfinder to use.
    let gl_context = ContextBuilder::new().with_gl(GlRequest::Latest)
                                          .with_gl_profile(GlProfile::Core)
                                          .build_windowed(window_builder, &event_loop)
                                          .unwrap();

    // Load OpenGL, and make the context current.
    let gl_context = unsafe { gl_context.make_current().unwrap() };
    gl::load_with(|name| gl_context.get_proc_address(name) as *const _);

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(GLDevice::new(GLVersion::GL3, 0),
                                     &FilesystemResourceLoader::locate(),
                                     DestFramebuffer::full_window(window_size),
                                     RendererOptions { background_color: Some(ColorF::white()) });

    // Make a canvas. We're going to draw a house.
    let font_context = CanvasFontContext::from_system_source();
    let mut canvas = Canvas::new(window_size.to_f32()).get_context_2d(font_context);

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
    gl_context.swap_buffers().unwrap();

    // Wait for a keypress.
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } |
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. },
                    ..
                },
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            },
            _ => {
                *control_flow = ControlFlow::Wait;
            },
        };
    })
}
