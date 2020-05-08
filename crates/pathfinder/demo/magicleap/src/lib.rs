// pathfinder/demo/magicleap/src/lib.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A demo app for Pathfinder on ML1.

use crate::magicleap::MagicLeapLogger;
use crate::magicleap::MagicLeapWindow;

use egl;
use egl::EGLContext;
use egl::EGLDisplay;
use egl::EGLSurface;

use gl::types::GLuint;

use log::info;

use pathfinder_demo::DemoApp;
use pathfinder_demo::Options;
use pathfinder_demo::UIVisibility;
use pathfinder_demo::BackgroundColor;
use pathfinder_demo::Mode;
use pathfinder_demo::window::Event;
use pathfinder_demo::window::SVGPath;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_geometry::vector::vec2i;
use pathfinder_color::ColorF;
use pathfinder_gl::GLDevice;
use pathfinder_gl::GLVersion;
use pathfinder_gpu::ClearParams;
use pathfinder_gpu::Device;
use pathfinder_renderer::concurrent::executor::SequentialExecutor;
use pathfinder_renderer::concurrent::scene_proxy::SceneProxy;
use pathfinder_renderer::gpu::renderer::Renderer;
use pathfinder_renderer::gpu::renderer::DestFramebuffer;
use pathfinder_renderer::options::RenderOptions;
use pathfinder_renderer::options::RenderTransform;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_simd::default::F32x4;
use pathfinder_svg::BuiltSVG;

use std::collections::HashMap;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;
use std::os::raw::c_void;

use usvg::Options as UsvgOptions;
use usvg::Tree;

mod c_api;
mod magicleap;

#[cfg(feature = "mocked")]
mod mocked_c_api;

struct ImmersiveApp {
    sender: crossbeam_channel::Sender<Event>,
    receiver: crossbeam_channel::Receiver<Event>,
    demo: DemoApp<MagicLeapWindow>,
}

#[no_mangle]
pub extern "C" fn magicleap_pathfinder_demo_init(egl_display: EGLDisplay, egl_context: EGLContext) -> *mut c_void {
    unsafe { c_api::MLLoggingLog(c_api::MLLogLevel::Info,
                                 b"Pathfinder Demo\0".as_ptr() as *const _,
                                 b"Initializing\0".as_ptr() as *const _) };

    let tag = CString::new("Pathfinder Demo").unwrap();
    let level = log::LevelFilter::Warn;
    let logger = MagicLeapLogger::new(tag, level);
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(level);
    info!("Initialized logging");

    let window = MagicLeapWindow::new(egl_display, egl_context);
    let window_size = window.size();

    let mut options = Options::default();
    options.ui = UIVisibility::None;
    options.background_color = BackgroundColor::Transparent;
    options.mode = Mode::VR;
    options.jobs = Some(3);

    let demo = DemoApp::new(window, window_size, options);
    info!("Initialized app");

    let (sender, receiver) = crossbeam_channel::unbounded();
    Box::into_raw(Box::new(ImmersiveApp { sender, receiver, demo })) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn magicleap_pathfinder_demo_run(app: *mut c_void) {
    let app = app as *mut ImmersiveApp;
    if let Some(app) = app.as_mut() {
        while app.demo.window.running() {
            let mut events = Vec::new();
            while let Some(event) = app.demo.window.try_get_event() {
                events.push(event);
            }
            while let Ok(event) = app.receiver.try_recv() {
                events.push(event);
            }
            let scene_count = app.demo.prepare_frame(events);
            app.demo.draw_scene();
            app.demo.begin_compositing();
            for scene_index in 0..scene_count {
                app.demo.composite_scene(scene_index);
            }
            app.demo.finish_drawing_frame();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn magicleap_pathfinder_demo_load(app: *mut c_void, svg_filename: *const c_char) {
    let app = app as *mut ImmersiveApp;
    if let Some(app) = app.as_mut() {
        let svg_filename = CStr::from_ptr(svg_filename).to_string_lossy().into_owned();
        info!("Loading {}.", svg_filename);
        let _ = app.sender.send(Event::OpenSVG(SVGPath::Resource(svg_filename)));
    }
}

struct MagicLeapPathfinder {
    renderers: HashMap<(EGLSurface, EGLDisplay), Renderer<GLDevice>>,
    svgs: HashMap<String, BuiltSVG>,
    resources: FilesystemResourceLoader,
}

#[repr(C)]
pub struct MagicLeapPathfinderRenderOptions {
    display: EGLDisplay,
    surface: EGLSurface,
    bg_color: [f32; 4],
    viewport: [u32; 4],
    svg_filename: *const c_char,
}

#[no_mangle]
pub extern "C" fn magicleap_pathfinder_init() -> *mut c_void {
    unsafe { c_api::MLLoggingLog(c_api::MLLogLevel::Info,
                                 b"Pathfinder Demo\0".as_ptr() as *const _,
                                 b"Initializing\0".as_ptr() as *const _) };

    let tag = CString::new("Pathfinder Demo").unwrap();
    let level = log::LevelFilter::Info;
    let logger = MagicLeapLogger::new(tag, level);
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(level);
    info!("Initialized logging");

    gl::load_with(|s| egl::get_proc_address(s) as *const c_void);
    info!("Initialized gl");

    let pf = MagicLeapPathfinder {
        renderers: HashMap::new(),
        svgs: HashMap::new(),
        resources: FilesystemResourceLoader::locate(),
    };
    info!("Initialized pf");

    Box::into_raw(Box::new(pf)) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn magicleap_pathfinder_render(pf: *mut c_void, options: *const MagicLeapPathfinderRenderOptions) {
    let pf = pf as *mut MagicLeapPathfinder;
    if let (Some(pf), Some(options)) = (pf.as_mut(), options.as_ref()) {
        let resources = &pf.resources;

        let svg_filename = CStr::from_ptr(options.svg_filename).to_string_lossy().into_owned();
        let svg = pf.svgs.entry(svg_filename).or_insert_with(|| {
            let svg_filename = CStr::from_ptr(options.svg_filename).to_string_lossy();
            let data = resources.slurp(&*svg_filename).unwrap();
            let tree = Tree::from_data(&data, &UsvgOptions::default()).unwrap();
            BuiltSVG::from_tree(tree)
        });

        let mut width = 0;
        let mut height = 0;
        egl::query_surface(options.display, options.surface, egl::EGL_WIDTH, &mut width);
        egl::query_surface(options.display, options.surface, egl::EGL_HEIGHT, &mut height);
        let size = vec2i(width, height);

        let viewport_origin = vec2i(options.viewport[0] as i32, options.viewport[1] as i32);
        let viewport_size = vec2i(options.viewport[2] as i32, options.viewport[3] as i32);
        let viewport = RectI::new(viewport_origin, viewport_size);

        let bg_color = ColorF(F32x4::new(options.bg_color[0], options.bg_color[1], options.bg_color[2], options.bg_color[3]));

        let renderer = pf.renderers.entry((options.display, options.surface)).or_insert_with(|| {
            let mut fbo = 0;
            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut fbo);
            let device = GLDevice::new(GLVersion::GLES3, fbo as GLuint);
            let dest_framebuffer = DestFramebuffer::Default { viewport, window_size: size };
            Renderer::new(device, resources, dest_framebuffer)
        });

        renderer.set_main_framebuffer_size(size);
        renderer.device.bind_default_framebuffer(viewport);
        renderer.device.clear(&ClearParams { color: Some(bg_color), ..ClearParams::default() });
        renderer.disable_depth();

        svg.scene.set_view_box(viewport.to_f32());

        let scale = i32::min(viewport_size.x(), viewport_size.y()) as f32 /
            f32::max(svg.scene.bounds().size().x(), svg.scene.bounds().size().y());
        let transform = Transform2F::from_translation(svg.scene.bounds().size().scale(-0.5))
            .post_mul(&Transform2F::from_scale(scale))
            .post_mul(&Transform2F::from_translation(viewport_size.to_f32().scale(0.5)));

        let render_options = RenderOptions {
            transform: RenderTransform::Transform2D(transform),
            dilation: Vector2F::zero(),
            subpixel_aa_enabled: false,
        };

        let scene_proxy = SceneProxy::from_scene(svg.scene.clone(), SequentialExecutor);
        scene_proxy.build_and_render(renderer, render_options);
    }
}

#[no_mangle]
pub unsafe extern "C" fn magicleap_pathfinder_deinit(pf: *mut c_void) {
    Box::from_raw(pf as *mut MagicLeapPathfinder);
}
