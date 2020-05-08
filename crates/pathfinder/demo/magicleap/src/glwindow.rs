// pathfinder/demo/immersive/glwindow.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use gl;

use glutin::ContextBuilder;
use glutin::ContextError;
use glutin::CreationError;
use glutin::EventsLoop;
use glutin::Event;
use glutin::WindowEvent;
use glutin::GlContext;
use glutin::GlWindow;
use glutin::WindowBuilder;
use glutin::dpi::LogicalSize;

use crate::display::Display;
use crate::display::DisplayCamera;
use crate::display::DisplayError;

use pathfinder_geometry::point::Point2DI32;
use pathfinder_geometry::rect::RectI32;
use pathfinder_geometry::transform3d::Transform4F32;
use pathfinder_geometry::transform3d::Perspective;
use pathfinder_gl::GLVersion;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;

use std::env;
use std::error::Error;
use std::fmt;
use std::f32::consts::FRAC_PI_4;
use std::io;
use std::rc::Rc;
use std::time::Instant;

use usvg;

pub struct GlWindowDisplay {
    events_loop: EventsLoop,
    gl_window: Rc<GlWindow>,
    running: bool,
    cameras: Vec<GlWindowCamera>,
    resource_loader: FilesystemResourceLoader,
}

pub struct GlWindowCamera {
    eye: Eye,
    gl_window: Rc<GlWindow>,
    start: Instant,
}

enum Eye {
    Left,
    Right,
}

#[derive(Debug)]
pub enum GlWindowError {
    Creation(CreationError),
    Context(ContextError),
    SVG(usvg::Error),
    IO(io::Error),
}

const DEFAULT_EYE_WIDTH: u32 = 1024;
const DEFAULT_EYE_HEIGHT: u32 = 768;

const CAMERA_DISTANCE: f32 = 3.0;
const NEAR_CLIP_PLANE: f32 = 0.01;
const FAR_CLIP_PLANE:  f32 = 10.0;

impl Display for GlWindowDisplay {
    type Error = GlWindowError;
    type Camera = GlWindowCamera;

    fn resource_loader(&self) -> &dyn ResourceLoader {
        &self.resource_loader
    }

    fn gl_version(&self) -> GLVersion {
        GLVersion::GL3
    }

    fn make_current(&mut self) -> Result<(), GlWindowError> {
        let size = self.size();
        unsafe {
            self.gl_window.make_current()?;
            gl::Viewport(0, 0, size.x(), size.y());
            gl::Scissor(0, 0, size.x(), size.y());
            gl::Enable(gl::SCISSOR_TEST);
        }
        self.handle_events();
        Ok(())
    }

    fn begin_frame(&mut self) -> Result<&mut[GlWindowCamera], GlWindowError> {
        self.handle_events();
        Ok(&mut self.cameras[..])
    }

    fn end_frame(&mut self) -> Result<(), GlWindowError> {
        self.handle_events();
        self.gl_window.swap_buffers()?;
        self.handle_events();
        Ok(())
    }

    fn running(&self) -> bool {
        self.running
    }

    fn size(&self) -> Point2DI32 {
        window_size(&*self.gl_window)
    }
}

impl DisplayCamera for GlWindowCamera {
    type Error = GlWindowError;

    fn make_current(&mut self) -> Result<(), GlWindowError> {
        let bounds = self.bounds();
        unsafe {
            self.gl_window.make_current()?;
            gl::Viewport(bounds.origin().x(), bounds.origin().y(), bounds.size().x(), bounds.size().y());
            gl::Scissor(bounds.origin().x(), bounds.origin().y(), bounds.size().x(), bounds.size().y());
        }
        Ok(())
    }

    fn bounds(&self) -> RectI32 {
        let window_size = window_size(&*self.gl_window);
        let eye_size = Point2DI32::new(window_size.x()/2, window_size.y());
        let origin = match self.eye {
	    Eye::Left => Point2DI32::new(0, 0),
	    Eye::Right => Point2DI32::new(eye_size.x(), 0),
	};
        RectI32::new(origin, eye_size)
    }

    fn perspective(&self) -> Perspective {
        // TODO: add eye offsets
        let bounds = self.bounds();
        let aspect = bounds.size().x() as f32 / bounds.size().y() as f32;
        let transform = Transform4F32::from_perspective(FRAC_PI_4, aspect, NEAR_CLIP_PLANE, FAR_CLIP_PLANE);
        Perspective::new(&transform, bounds.size())
    }

    fn view(&self) -> Transform4F32 {
        let duration = Instant::now() - self.start;
        let rotation = duration.as_millis() as f32 / 1000.0;
        Transform4F32::from_rotation(rotation, 0.0, 0.0)
            .pre_mul(&Transform4F32::from_translation(0.0, 0.0, -CAMERA_DISTANCE))
    }
}

impl GlWindowDisplay {
    pub fn new() -> Result<GlWindowDisplay, GlWindowError> {
        let resource_loader = FilesystemResourceLoader::locate();
        let size = default_window_size();
        let events_loop = glutin::EventsLoop::new();
        let window = WindowBuilder::new()
            .with_title("Pathfinder Immersive Demo")
            .with_dimensions(size);
        let context = ContextBuilder::new()
            .with_vsync(true);
        let gl_window = Rc::new(glutin::GlWindow::new(window, context, &events_loop)?);
	let start = Instant::now();
	let cameras = vec![
	    GlWindowCamera { gl_window: gl_window.clone(), start, eye: Eye::Left },
	    GlWindowCamera { gl_window: gl_window.clone(), start, eye: Eye::Right },
        ];
        gl::load_with(|name| gl_window.get_proc_address(name) as *const _);
        Ok(GlWindowDisplay {
	    resource_loader,
            events_loop,
            gl_window,
	    cameras,
            running: true,
        })
    }

    fn handle_events(&mut self) {
        let running = &mut self.running;
        self.events_loop.poll_events(|event| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } |
                Event::WindowEvent { event: WindowEvent::Destroyed, .. } => *running = false,
                _ => (),
            }
        })
    }
}

fn window_size(gl_window: &GlWindow) -> Point2DI32 {
    let logical = gl_window
        .get_inner_size()
        .unwrap_or_else(|| default_window_size());
    let hidpi = gl_window.get_hidpi_factor();
    let physical = logical.to_physical(hidpi);
    Point2DI32::new(physical.width as i32, physical.height as i32)
}

fn default_window_size() -> LogicalSize {
    LogicalSize::new((DEFAULT_EYE_WIDTH * 2) as f64, DEFAULT_EYE_HEIGHT as f64)
}

impl fmt::Display for GlWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            GlWindowError::Creation(ref err) => err.fmt(formatter),
            GlWindowError::Context(ref err) => err.fmt(formatter),
            GlWindowError::SVG(ref err) => err.fmt(formatter),
            GlWindowError::IO(ref err) => err.fmt(formatter),
        }
    }
}

impl Error for GlWindowError {
}

impl From<CreationError> for GlWindowError {
    fn from(err: CreationError) -> GlWindowError {
        GlWindowError::Creation(err)
    }
}

impl From<ContextError> for GlWindowError {
    fn from(err: ContextError) -> GlWindowError {
        GlWindowError::Context(err)
    }
}

impl From<usvg::Error> for GlWindowError {
    fn from(err: usvg::Error) -> GlWindowError {
        GlWindowError::SVG(err)
    }
}

impl From<io::Error> for GlWindowError {
    fn from(err: io::Error) -> GlWindowError {
        GlWindowError::IO(err)
    }
}

impl DisplayError for GlWindowError {
}
