// pathfinder/examples/canvas_metal_minimal/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use foreign_types::ForeignTypeRef;
use metal::{CAMetalLayer, CoreAnimationLayerRef};
use pathfinder_canvas::{CanvasFontContext, CanvasRenderingContext2D, Path2D};
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f, vec2i};
use pathfinder_geometry::rect::RectF;
use pathfinder_metal::MetalDevice;
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::Event;
use sdl2::hint;
use sdl2::keyboard::Keycode;
use sdl2_sys::SDL_RenderGetMetalLayer;

fn main() {
    // Set up SDL2.
    assert!(hint::set("SDL_RENDER_DRIVER", "metal"));
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Open a window.
    let window_size = vec2i(640, 480);
    let window = video.window("Minimal example", window_size.x() as u32, window_size.y() as u32)
                      .opengl()
                      .build()
                      .unwrap();

    // Create a Metal context.
    let canvas = window.into_canvas().present_vsync().build().unwrap();
    let metal_layer = unsafe {
        CoreAnimationLayerRef::from_ptr(SDL_RenderGetMetalLayer(canvas.raw()) as *mut CAMetalLayer)
    };

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(MetalDevice::new(metal_layer),
                                     &FilesystemResourceLoader::locate(),
                                     DestFramebuffer::full_window(window_size),
                                     RendererOptions { background_color: Some(ColorF::white()) });

    // Make a canvas. We're going to draw a house.
    let mut canvas = CanvasRenderingContext2D::new(CanvasFontContext::from_system_source(),
                                                   window_size.to_f32());

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
    let scene = SceneProxy::from_scene(canvas.into_scene(), RayonExecutor);
    scene.build_and_render(&mut renderer, BuildOptions::default());
    renderer.device.present_drawable();

    // Wait for a keypress.
    let mut event_pump = sdl_context.event_pump().unwrap();
    loop {
        match event_pump.wait_event() {
            Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return,
            _ => {}
        }
    }
}
