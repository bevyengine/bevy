// pathfinder/examples/canvas_moire/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use pathfinder_canvas::{Canvas, CanvasFontContext, CanvasRenderingContext2D, FillStyle, Path2D};
use pathfinder_color::{ColorF, ColorU};
use pathfinder_geometry::vector::{Vector2F, Vector2I, vec2f, vec2i};
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::concurrent::rayon::RayonExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::options::{DestFramebuffer, RendererOptions};
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::options::BuildOptions;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::f32::consts::PI;
use std::f32;

const VELOCITY: f32 = 0.02;
const OUTER_RADIUS: f32 = 64.0;
const INNER_RADIUS: f32 = 48.0;

// FIXME(pcwalton): Adding more circles causes clipping problems. Fix them!
const CIRCLE_COUNT: u32 = 12;

const CIRCLE_SPACING: f32 = 48.0;
const CIRCLE_THICKNESS: f32 = 16.0;

const COLOR_CYCLE_SPEED: f32 = 0.0025;

fn main() {
    // Set up SDL2.
    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();

    // Make sure we have at least a GL 3.0 context. Pathfinder requires this.
    let gl_attributes = video.gl_attr();
    gl_attributes.set_context_profile(GLProfile::Core);
    gl_attributes.set_context_version(3, 3);

    // Open a window.
    let window_size = vec2i(1067, 800);
    let window = video.window("Moire example", window_size.x() as u32, window_size.y() as u32)
                      .opengl()
                      .allow_highdpi()
                      .build()
                      .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // Get the real window size (for HiDPI).
    let (drawable_width, drawable_height) = window.drawable_size();
    let drawable_size = vec2i(drawable_width as i32, drawable_height as i32);

    // Create the GL context, and make it current.
    let gl_context = window.gl_create_context().unwrap();
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    window.gl_make_current(&gl_context).unwrap();

    // Create our renderers.
    let renderer = Renderer::new(GLDevice::new(GLVersion::GL3, 0),
                                 &FilesystemResourceLoader::locate(),
                                 DestFramebuffer::full_window(drawable_size),
                                 RendererOptions { background_color: Some(ColorF::white()) });
    let mut moire_renderer = MoireRenderer::new(renderer, window_size, drawable_size);

    // Enter main render loop.
    loop {
        moire_renderer.render();
        window.gl_swap_window();

        match event_pump.poll_event() {
            Some(Event::Quit {..}) |
            Some(Event::KeyDown { keycode: Some(Keycode::Escape), .. }) => return,
            _ => {}
        }
    }
}

struct MoireRenderer {
    renderer: Renderer<GLDevice>,
    font_context: CanvasFontContext,
    scene: SceneProxy,
    frame: i32,
    window_size: Vector2I,
    drawable_size: Vector2I,
    device_pixel_ratio: f32,
    colors: ColorGradient,
}

impl MoireRenderer {
    fn new(renderer: Renderer<GLDevice>, window_size: Vector2I, drawable_size: Vector2I)
           -> MoireRenderer {
        MoireRenderer {
            renderer,
            font_context: CanvasFontContext::from_system_source(),
            scene: SceneProxy::new(RayonExecutor),
            frame: 0,
            window_size,
            drawable_size,
            device_pixel_ratio: drawable_size.x() as f32 / window_size.x() as f32,
            colors: ColorGradient::new(),
        }
    }

    fn render(&mut self) {
        // Calculate animation values.
        let time = self.frame as f32;
        let (sin_time, cos_time) = (f32::sin(time * VELOCITY), f32::cos(time * VELOCITY));
        let color_time = time * COLOR_CYCLE_SPEED;
        let background_color = self.colors.sample(color_time);
        let foreground_color = self.colors.sample(color_time + 0.5);

        // Calculate outer and inner circle centers (circle and Leminscate of Gerono respectively).
        let window_center = self.window_size.to_f32() * 0.5;
        let outer_center = window_center + vec2f(sin_time, cos_time) * OUTER_RADIUS;
        let inner_center = window_center + vec2f(1.0, sin_time) * (cos_time * INNER_RADIUS);

        // Clear to background color.
        self.renderer.set_options(RendererOptions { background_color: Some(background_color) });

        // Make a canvas.
        let mut canvas =    
            Canvas::new(self.drawable_size.to_f32()).get_context_2d(self.font_context.clone());
        canvas.set_line_width(CIRCLE_THICKNESS * self.device_pixel_ratio);
        canvas.set_stroke_style(FillStyle::Color(foreground_color.to_u8()));
        canvas.set_global_alpha(0.75);

        // Draw circles.
        self.draw_circles(&mut canvas, outer_center);
        self.draw_circles(&mut canvas, inner_center);

        // Build and render scene.
        self.scene.replace_scene(canvas.into_canvas().into_scene());
        self.scene.build_and_render(&mut self.renderer, BuildOptions::default());

        self.frame += 1;
    }

    fn draw_circles(&self, canvas: &mut CanvasRenderingContext2D, mut center: Vector2F) {
        center *= self.device_pixel_ratio;
        for index in 0..CIRCLE_COUNT {
            let radius = (index + 1) as f32 * CIRCLE_SPACING * self.device_pixel_ratio;
            let mut path = Path2D::new();
            path.ellipse(center, radius, 0.0, 0.0, PI * 2.0);
            canvas.stroke_path(path);
        }
    }
}

struct ColorGradient([ColorF; 5]);

impl ColorGradient {
    fn new() -> ColorGradient {
        // Extracted from https://stock.adobe.com/69426938/
        ColorGradient([
            ColorU::from_u32(0x024873ff).to_f32(),
            ColorU::from_u32(0x03658cff).to_f32(),
            ColorU::from_u32(0x0388a6ff).to_f32(),
            ColorU::from_u32(0xf28e6bff).to_f32(),
            ColorU::from_u32(0xd95a4eff).to_f32(),
        ])
    }

    fn sample(&self, mut t: f32) -> ColorF {
        let count = self.0.len();
        t *= count as f32;
        let (lo, hi) = (t.floor() as usize % count, t.ceil() as usize % count);
        self.0[lo].lerp(self.0[hi], f32::fract(t))
    }
}
