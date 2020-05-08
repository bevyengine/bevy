// pathfinder/demo/native/src/main.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A demo app for Pathfinder using SDL 2.

use nfd::Response;
use pathfinder_demo::window::{Event, Keycode, SVGPath, View, Window, WindowSize};
use pathfinder_demo::{DemoApp, Options};
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::vec2i;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use sdl2::event::{Event as SDLEvent, WindowEvent};
use sdl2::keyboard::Keycode as SDLKeycode;
use sdl2::video::Window as SDLWindow;
use sdl2::{EventPump, EventSubsystem, Sdl, VideoSubsystem};
use sdl2_sys::{SDL_Event, SDL_UserEvent};
use std::path::PathBuf;
use std::ptr;

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use foreign_types::ForeignTypeRef;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use metal::{CAMetalLayer, CoreAnimationLayerRef};
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use pathfinder_metal::MetalDevice;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use sdl2::hint;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use sdl2::render::Canvas;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use sdl2_sys::SDL_RenderGetMetalLayer;

#[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
use pathfinder_gl::{GLDevice, GLVersion};
#[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
use sdl2::video::{GLContext, GLProfile};

#[cfg(not(windows))]
use jemallocator;

#[cfg(not(windows))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const DEFAULT_WINDOW_WIDTH: u32 = 1067;
const DEFAULT_WINDOW_HEIGHT: u32 = 800;

fn main() {
    color_backtrace::install();
    pretty_env_logger::init();

    let window = WindowImpl::new();
    let window_size = window.size();
    let options = Options::default();
    let mut app = DemoApp::new(window, window_size, options);

    while !app.should_exit {
        let mut events = vec![];
        if !app.dirty {
            events.push(app.window.get_event());
        }
        while let Some(event) = app.window.try_get_event() {
            events.push(event);
        }

        let scene_count = app.prepare_frame(events);
        app.draw_scene();
        app.begin_compositing();
        for scene_index in 0..scene_count {
            app.composite_scene(scene_index);
        }
        app.finish_drawing_frame();
    }
}

thread_local! {
    static SDL_CONTEXT: Sdl = sdl2::init().unwrap();
    static SDL_VIDEO: VideoSubsystem = SDL_CONTEXT.with(|context| context.video().unwrap());
    static SDL_EVENT: EventSubsystem = SDL_CONTEXT.with(|context| context.event().unwrap());
}

struct WindowImpl {
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    window: SDLWindow,
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    gl_context: GLContext,

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    canvas: Canvas<SDLWindow>,
    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    metal_layer: *mut CAMetalLayer,

    event_pump: EventPump,
    #[allow(dead_code)]
    resource_loader: FilesystemResourceLoader,
    selected_file: Option<PathBuf>,
    open_svg_message_type: u32,
}

impl Window for WindowImpl {
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn gl_version(&self) -> GLVersion {
        GLVersion::GL3
    }

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn metal_layer(&self) -> &CoreAnimationLayerRef {
        unsafe { CoreAnimationLayerRef::from_ptr(self.metal_layer) }
    }

    fn viewport(&self, view: View) -> RectI {
        let (width, height) = self.window().drawable_size();
        let mut width = width as i32;
        let height = height as i32;
        let mut x_offset = 0;
        if let View::Stereo(index) = view {
            width = width / 2;
            x_offset = width * (index as i32);
        }
        RectI::new(vec2i(x_offset, 0), vec2i(width, height))
    }

    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn make_current(&mut self, _view: View) {
        self.window().gl_make_current(&self.gl_context).unwrap();
    }

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn make_current(&mut self, _: View) {}

    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn present(&mut self, _: &mut GLDevice) {
        self.window().gl_swap_window();
    }

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn present(&mut self, device: &mut MetalDevice) {
        device.present_drawable();
    }

    fn resource_loader(&self) -> &dyn ResourceLoader {
        &self.resource_loader
    }

    fn create_user_event_id(&self) -> u32 {
        SDL_EVENT.with(|sdl_event| unsafe { sdl_event.register_event().unwrap() })
    }

    fn push_user_event(message_type: u32, message_data: u32) {
        unsafe {
            let mut user_event = SDL_UserEvent {
                timestamp: 0,
                windowID: 0,
                type_: message_type,
                code: message_data as i32,
                data1: ptr::null_mut(),
                data2: ptr::null_mut(),
            };
            sdl2_sys::SDL_PushEvent(&mut user_event as *mut SDL_UserEvent as *mut SDL_Event);
        }
    }

    fn present_open_svg_dialog(&mut self) {
        if let Ok(Response::Okay(path)) = nfd::open_file_dialog(Some("svg"), None) {
            self.selected_file = Some(PathBuf::from(path));
            WindowImpl::push_user_event(self.open_svg_message_type, 0);
        }
    }

    fn run_save_dialog(&self, extension: &str) -> Result<PathBuf, ()> {
        match nfd::open_save_dialog(Some(extension), None) {
            Ok(Response::Okay(file)) => Ok(PathBuf::from(file)),
            _ => Err(()),
        }
    }
}

impl WindowImpl {
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn new() -> WindowImpl {
        SDL_VIDEO.with(|sdl_video| {
            SDL_EVENT.with(|sdl_event| {
                let (window, gl_context, event_pump);

                let gl_attributes = sdl_video.gl_attr();
                gl_attributes.set_context_profile(GLProfile::Core);
                gl_attributes.set_context_version(3, 3);
                gl_attributes.set_depth_size(24);
                gl_attributes.set_stencil_size(8);

                window = sdl_video
                    .window(
                        "Pathfinder Demo",
                        DEFAULT_WINDOW_WIDTH,
                        DEFAULT_WINDOW_HEIGHT,
                    )
                    .opengl()
                    .resizable()
                    .allow_highdpi()
                    .build()
                    .unwrap();

                gl_context = window.gl_create_context().unwrap();
                gl::load_with(|name| sdl_video.gl_get_proc_address(name) as *const _);

                event_pump = SDL_CONTEXT.with(|sdl_context| sdl_context.event_pump().unwrap());

                let resource_loader = FilesystemResourceLoader::locate();

                let open_svg_message_type = unsafe { sdl_event.register_event().unwrap() };

                WindowImpl {
                    window,
                    event_pump,
                    gl_context,
                    resource_loader,
                    open_svg_message_type,
                    selected_file: None,
                }
            })
        })
    }

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn new() -> WindowImpl {
        assert!(hint::set("SDL_RENDER_DRIVER", "metal"));

        SDL_VIDEO.with(|sdl_video| {
            SDL_EVENT.with(|sdl_event| {
                let window = sdl_video
                    .window(
                        "Pathfinder Demo",
                        DEFAULT_WINDOW_WIDTH,
                        DEFAULT_WINDOW_HEIGHT,
                    )
                    .opengl()
                    .resizable()
                    .allow_highdpi()
                    .build()
                    .unwrap();

                let canvas = window.into_canvas().present_vsync().build().unwrap();
                let metal_layer = unsafe {
                    SDL_RenderGetMetalLayer(canvas.raw()) as *mut CAMetalLayer
                };

                let event_pump = SDL_CONTEXT.with(|sdl_context| sdl_context.event_pump().unwrap());

                let resource_loader = FilesystemResourceLoader::locate();

                let open_svg_message_type = unsafe { sdl_event.register_event().unwrap() };

                WindowImpl {
                    event_pump,
                    canvas,
                    metal_layer,
                    resource_loader,
                    open_svg_message_type,
                    selected_file: None,
                }
            })
        })
    }

    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn window(&self) -> &SDLWindow { &self.window }
    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn window(&self) -> &SDLWindow { self.canvas.window() }

    fn size(&self) -> WindowSize {
        let (logical_width, logical_height) = self.window().size();
        let (drawable_width, _) = self.window().drawable_size();
        WindowSize {
            logical_size: vec2i(logical_width as i32, logical_height as i32),
            backing_scale_factor: drawable_width as f32 / logical_width as f32,
        }
    }

    fn get_event(&mut self) -> Event {
        loop {
            let sdl_event = self.event_pump.wait_event();
            if let Some(event) = self.convert_sdl_event(sdl_event) {
                return event;
            }
        }
    }

    fn try_get_event(&mut self) -> Option<Event> {
        loop {
            let sdl_event = self.event_pump.poll_event()?;
            if let Some(event) = self.convert_sdl_event(sdl_event) {
                return Some(event);
            }
        }
    }

    fn convert_sdl_event(&self, sdl_event: SDLEvent) -> Option<Event> {
        match sdl_event {
            SDLEvent::User { type_, .. } if type_ == self.open_svg_message_type => Some(
                Event::OpenSVG(SVGPath::Path(self.selected_file.clone().unwrap())),
            ),
            SDLEvent::User { type_, code, .. } => Some(Event::User {
                message_type: type_,
                message_data: code as u32,
            }),
            SDLEvent::MouseButtonDown { x, y, .. } => Some(Event::MouseDown(vec2i(x, y))),
            SDLEvent::MouseMotion {
                x, y, mousestate, ..
            } => {
                let position = vec2i(x, y);
                if mousestate.left() {
                    Some(Event::MouseDragged(position))
                } else {
                    Some(Event::MouseMoved(position))
                }
            }
            SDLEvent::Quit { .. } => Some(Event::Quit),
            SDLEvent::Window {
                win_event: WindowEvent::SizeChanged(..),
                ..
            } => Some(Event::WindowResized(self.size())),
            SDLEvent::KeyDown {
                keycode: Some(sdl_keycode),
                ..
            } => self.convert_sdl_keycode(sdl_keycode).map(Event::KeyDown),
            SDLEvent::KeyUp {
                keycode: Some(sdl_keycode),
                ..
            } => self.convert_sdl_keycode(sdl_keycode).map(Event::KeyUp),
            SDLEvent::MultiGesture { d_dist, .. } => {
                let mouse_state = self.event_pump.mouse_state();
                let center = vec2i(mouse_state.x(), mouse_state.y());
                Some(Event::Zoom(d_dist, center))
            }
            _ => None,
        }
    }

    fn convert_sdl_keycode(&self, sdl_keycode: SDLKeycode) -> Option<Keycode> {
        match sdl_keycode {
            SDLKeycode::Escape => Some(Keycode::Escape),
            SDLKeycode::Tab => Some(Keycode::Tab),
            sdl_keycode
                if sdl_keycode as i32 >= SDLKeycode::A as i32
                    && sdl_keycode as i32 <= SDLKeycode::Z as i32 =>
            {
                let offset = (sdl_keycode as i32 - SDLKeycode::A as i32) as u8;
                Some(Keycode::Alphanumeric(offset + b'a'))
            }
            _ => None,
        }
    }
}
