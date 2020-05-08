// pathfinder/examples/canvas_text/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use font_kit::handle::Handle;
use pathfinder_canvas::{Canvas, CanvasFontContext, TextAlign};
use pathfinder_color::ColorF;
use pathfinder_geometry::vector::{vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::iter;
use std::sync::Arc;

fn main() {
    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // Open a window.
    let window_size = vec2i(640, 480);
    let window = video.window("Text example", window_size.x() as u32, window_size.y() as u32)
                      .opengl()
                      .build()
                      .unwrap();

    // Create the GL context, and make it current.
    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    // Create a Pathfinder renderer.
    let resource_loader = FilesystemResourceLoader::locate();
    let mut renderer = Renderer::new(GLDevice::new(GLVersion::GL3, 0),
                                     &resource_loader,
                                     DestFramebuffer::full_window(window_size),
                                     RendererOptions { background_color: Some(ColorF::white()) });

    // Load a font.
    let font_data = Arc::new(resource_loader.slurp("fonts/Overpass-Regular.otf").unwrap());
    let font = Handle::from_memory(font_data, 0);
    let font_context = CanvasFontContext::from_fonts(iter::once(font));

    // Make a canvas.
    let mut canvas = Canvas::new(window_size.to_f32()).get_context_2d(font_context);

    // Draw the text.
    canvas.set_font("Overpass-Regular");
    canvas.set_font_size(32.0);
    canvas.fill_text("Hello Pathfinder!", vec2f(32.0, 48.0));
    canvas.set_text_align(TextAlign::Right);
    canvas.stroke_text("Goodbye Pathfinder!", vec2f(608.0, 464.0));

    // Render the canvas to screen.
    let scene = SceneProxy::from_scene(canvas.into_canvas().into_scene(), RayonExecutor);
    scene.build_and_render(&mut renderer, BuildOptions::default());
    window.gl_swap_window();

    // Wait for a keypress.
    let mut event_pump = sdl_context.event_pump().unwrap();
    loop {
        match event_pump.wait_event() {
            Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return,
            _ => {}
        }
    }
}
