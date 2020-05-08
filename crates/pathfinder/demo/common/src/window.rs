// pathfinder/demo/common/src/window.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A minimal cross-platform windowing layer.

use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform3d::{Perspective, Transform4F};
use pathfinder_geometry::vector::Vector2I;
use pathfinder_resources::ResourceLoader;
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;

#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use metal::CoreAnimationLayerRef;
#[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
use pathfinder_metal::MetalDevice;

#[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
use gl::types::GLuint;
#[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
use pathfinder_gl::{GLDevice, GLVersion};

pub trait Window {
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn gl_version(&self) -> GLVersion;
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn gl_default_framebuffer(&self) -> GLuint { 0 }
    #[cfg(any(not(target_os = "macos"), feature = "pf-gl"))]
    fn present(&mut self, device: &mut GLDevice);

    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn metal_layer(&self) -> &CoreAnimationLayerRef;
    #[cfg(all(target_os = "macos", not(feature = "pf-gl")))]
    fn present(&mut self, device: &mut MetalDevice);

    fn make_current(&mut self, view: View);
    fn viewport(&self, view: View) -> RectI;
    fn resource_loader(&self) -> &dyn ResourceLoader;
    fn create_user_event_id(&self) -> u32;
    fn push_user_event(message_type: u32, message_data: u32);
    fn present_open_svg_dialog(&mut self);
    fn run_save_dialog(&self, extension: &str) -> Result<PathBuf, ()>;

    fn adjust_thread_pool_settings(&self, builder: ThreadPoolBuilder) -> ThreadPoolBuilder {
        builder
    }
}

pub enum Event {
    Quit,
    WindowResized(WindowSize),
    KeyDown(Keycode),
    KeyUp(Keycode),
    MouseDown(Vector2I),
    MouseMoved(Vector2I),
    MouseDragged(Vector2I),
    Zoom(f32, Vector2I),
    Look {
        pitch: f32,
        yaw: f32,
    },
    SetEyeTransforms(Vec<OcularTransform>),
    OpenSVG(SVGPath),
    User {
        message_type: u32,
        message_data: u32,
    },
}

#[derive(Clone, Copy)]
pub enum Keycode {
    Alphanumeric(u8),
    Escape,
    Tab,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowSize {
    pub logical_size: Vector2I,
    pub backing_scale_factor: f32,
}

impl WindowSize {
    #[inline]
    pub fn device_size(&self) -> Vector2I {
        (self.logical_size.to_f32() * self.backing_scale_factor).to_i32()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum View {
    Mono,
    Stereo(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct OcularTransform {
    // The perspective which converts from camera coordinates to display coordinates
    pub perspective: Perspective,

    // The view transform which converts from world coordinates to camera coordinates
    pub modelview_to_eye: Transform4F,
}

#[derive(Clone)]
pub enum SVGPath {
    Default,
    Resource(String),
    Path(PathBuf),
}
