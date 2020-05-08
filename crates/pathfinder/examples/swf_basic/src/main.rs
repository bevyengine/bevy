// pathfinder/examples/swf_basic/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::options::{RenderTransform, BuildOptions};
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use pathfinder_renderer::scene::Scene;
use pathfinder_swf::{draw_paths_into_scene, process_swf_tags};
use std::env;
use std::fs::read;

fn main() {
    let resource_loader = FilesystemResourceLoader::locate();

    let swf_bytes;
    if let Some(path) = env::args().skip(1).next() {
        match read(path) {
            Ok(bytes) => {
                swf_bytes = bytes;
            },
            Err(e) => panic!(e)
        }
    } else {
        // NOTE(jon): This is a version of the ghostscript tiger graphic flattened to a single
        // layer with no overlapping shapes.  This is how artwork is 'natively' created in the Flash
        // authoring tool when an artist just draws directly onto the canvas (without 'object' mode
        // turned on, which is the default).
        // Subsequent shapes with different fills will knock out existing fills where they overlap.
        // A downside of this in current pathfinder is that cracks are visible between shape fills -
        // especially obvious if you set the context clear color to #ff00ff or similar.

        // Common speculation as to why the swf format stores vector graphics in this way says that
        // it is to save on file-size bytes, however in the case of our tiger, it results in a
        // larger file than the layered version, since the overlapping shapes and strokes create
        // a lot more geometry.  I think a more likely explanation for the choice is that it was
        // done to reduce overdraw in the software rasterizer running on late 90's era hardware?
        // Indeed, this mode gives pathfinders' occlusion culling pass nothing to do!

        // NOTE(jon): This is a version of the same graphic cut and pasted into the Flash authoring
        // tool from the SVG version loaded in Illustrator. When layered graphics are pasted
        // into Flash, by default they retain their layering, expressed as groups.
        // They are still presented as being on a single timeline layer.
        // They will be drawn back to front in much the same way as the SVG version.

        let default_tiger = resource_loader.slurp("swf/tiger.swf").unwrap();
        swf_bytes = Vec::from(&default_tiger[..]);
    }

    let (_, movie): (_, swf_types::Movie) =
        swf_parser::streaming::movie::parse_movie(&swf_bytes[..]).unwrap();

    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // process swf scene
    // TODO(jon): Since swf is a streaming format, this really wants to be a lazy iterator over
    // swf frames eventually.
    let (library, stage) = process_swf_tags(&movie);

    // Open a window.
    let window_size = vec2i(stage.width(), stage.height());
    let window = video.window("Minimal example", window_size.x() as u32, window_size.y() as u32)
        .opengl()
        .allow_highdpi()
        .build()
        .unwrap();

    let pixel_size = vec2i(window.drawable_size().0 as i32, window.drawable_size().1 as i32);
    let device_pixel_ratio = pixel_size.x() as f32 / window_size.x() as f32;

    // Create the GL context, and make it current.
    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    // Create a Pathfinder renderer.
    let mut renderer = Renderer::new(
        GLDevice::new(GLVersion::GL3, 0),
        &resource_loader,
        DestFramebuffer::full_window(pixel_size),
        RendererOptions { background_color: Some(stage.background_color()) }
    );
    // Clear to swf stage background color.
    let mut scene = Scene::new();
    scene.set_view_box(RectF::new(Vector2F::zero(),
                                  vec2f(stage.width() as f32,
                                        stage.height() as f32) * device_pixel_ratio));
    draw_paths_into_scene(&library, &mut scene);

    // Render the canvas to screen.
    let scene = SceneProxy::from_scene(scene, RayonExecutor);
    let mut build_options = BuildOptions::default();
    let scale_transform = Transform2F::from_scale(device_pixel_ratio);
    build_options.transform = RenderTransform::Transform2D(scale_transform);
    scene.build_and_render(&mut renderer, build_options);

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
