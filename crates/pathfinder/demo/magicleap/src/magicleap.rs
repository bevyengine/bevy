// pathfinder/demo/magicleap/src/magicleap.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::c_api::MLGraphicsBeginFrame;
use crate::c_api::MLGraphicsCreateClientGL;
use crate::c_api::MLGraphicsDestroyClient;
use crate::c_api::MLGraphicsEndFrame;
use crate::c_api::MLGraphicsGetRenderTargets;
use crate::c_api::MLGraphicsInitFrameParams;
use crate::c_api::MLGraphicsOptions;
use crate::c_api::MLGraphicsSignalSyncObjectGL;
use crate::c_api::MLGraphicsVirtualCameraInfoArray;
use crate::c_api::MLHandle;
use crate::c_api::MLHeadTrackingCreate;
use crate::c_api::MLLifecycleSetReadyIndication;
use crate::c_api::MLLogLevel;
use crate::c_api::MLLoggingLog;
use crate::c_api::MLMat4f;
use crate::c_api::MLQuaternionf;
use crate::c_api::MLRectf;
use crate::c_api::MLTransform;
use crate::c_api::MLVec3f;
use crate::c_api::ML_HANDLE_INVALID;
use crate::c_api::ML_RESULT_TIMEOUT;
use crate::c_api::ML_VIRTUAL_CAMERA_COUNT;

use egl;
use egl::EGL_NO_SURFACE;
use egl::EGLContext;
use egl::EGLDisplay;

use gl;
use gl::types::GLuint;

use log;
use log::debug;
use log::info;

use pathfinder_demo::window::Event;
use pathfinder_demo::window::OcularTransform;
use pathfinder_demo::window::View;
use pathfinder_demo::window::Window;
use pathfinder_demo::window::WindowSize;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::transform3d::Perspective;
use pathfinder_geometry::transform3d::Transform4F;
use pathfinder_geometry::util;
use pathfinder_geometry::vector::Vector2I;
use pathfinder_geometry::vector::Vector2F;
use pathfinder_geometry::vector::vec2i;
use pathfinder_gl::GLVersion;
use pathfinder_resources::ResourceLoader;
use pathfinder_resources::fs::FilesystemResourceLoader;
use pathfinder_simd::default::F32x4;

use rayon::ThreadPoolBuilder;

use smallvec::SmallVec;

use std::ffi::CString;
use std::io::Write;
use std::mem;
use std::os::raw::c_void;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub struct MagicLeapWindow {
    framebuffer_id: GLuint,
    graphics_client: MLHandle,
    size: Vector2I,
    virtual_camera_array: MLGraphicsVirtualCameraInfoArray,
    initial_camera_transform: Option<Transform4F>,
    frame_handle: MLHandle,
    resource_loader: FilesystemResourceLoader,
    pose_event: Option<Vec<OcularTransform>>,
    running: bool,
    in_frame: bool,
}

impl Window for MagicLeapWindow {
    fn resource_loader(&self) -> &dyn ResourceLoader {
        &self.resource_loader
    }

    fn gl_version(&self) -> GLVersion {
        GLVersion::GL3
    }

    fn gl_default_framebuffer(&self) -> GLuint {
        self.framebuffer_id
    }

    fn adjust_thread_pool_settings(&self, thread_pool_builder: ThreadPoolBuilder) -> ThreadPoolBuilder {
        thread_pool_builder.start_handler(|id| unsafe { init_scene_thread(id) })
    }

    fn create_user_event_id (&self) -> u32 {
        0
    }

    fn push_user_event(_: u32, _: u32) {
    }

    fn present_open_svg_dialog(&mut self) {
    }

    fn run_save_dialog(&self, _: &str) -> Result<PathBuf, ()> {
        Err(())
    }

    fn viewport(&self, _view: View) -> RectI {
        RectI::new(Vector2I::zero(), self.size)
    }

    fn make_current(&mut self, view: View) {
        self.begin_frame();
        let eye = match view {
            View::Stereo(eye) if (eye as usize) < ML_VIRTUAL_CAMERA_COUNT => eye as usize,
            _ => { debug!("Asked for unexpected view: {:?}", view); 0 }
        };
        debug!("Making {} current.", eye);
        let viewport = self.virtual_camera_array.viewport;
        let color_id = self.virtual_camera_array.color_id.as_gl_uint();
        let depth_id = self.virtual_camera_array.depth_id.as_gl_uint();
        let virtual_camera = self.virtual_camera_array.virtual_cameras[eye];
        let layer_id = virtual_camera.virtual_camera_name as i32;
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
            gl::FramebufferTextureLayer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, color_id, 0, layer_id);
            gl::FramebufferTextureLayer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, depth_id, 0, layer_id);
            gl::Viewport(viewport.x as i32, viewport.y as i32, viewport.w as i32, viewport.h as i32);
        }
        debug!("Made {} current.", eye);
    }

    fn present(&mut self) {
        self.end_frame();
        self.begin_frame();
    }
}

extern "C" {
    fn init_scene_thread(id: usize);
}

fn get_proc_address(s: &str) -> *const c_void {
    egl::get_proc_address(s) as *const c_void
}

impl MagicLeapWindow {
    pub fn new(egl_display: EGLDisplay, egl_context: EGLContext) -> MagicLeapWindow {
        debug!("Creating MagicLeapWindow");
        let mut framebuffer_id = 0;
        let graphics_options = MLGraphicsOptions::default();
        let mut graphics_client =  unsafe { mem::zeroed() };
        let mut head_tracker = unsafe { mem::zeroed() };
        let mut targets = unsafe { mem::zeroed() };
        let virtual_camera_array = unsafe { mem::zeroed() };
        let handle = MLHandle::from(egl_context);
        unsafe {
            egl::make_current(egl_display, EGL_NO_SURFACE, EGL_NO_SURFACE, egl_context);
            gl::load_with(get_proc_address);
            gl::GenFramebuffers(1, &mut framebuffer_id);
            MLGraphicsCreateClientGL(&graphics_options, handle, &mut graphics_client).unwrap();
            MLLifecycleSetReadyIndication().unwrap();
            MLHeadTrackingCreate(&mut head_tracker).unwrap();
            MLGraphicsGetRenderTargets(graphics_client, &mut targets).unwrap();
        }
        let (max_width, max_height) = targets.buffers.iter().map(|buffer| buffer.color)
            .chain(targets.buffers.iter().map(|buffer| buffer.depth))
            .map(|target| (target.width as i32, target.height as i32))
            .max()
            .unwrap_or_default();
        let resource_loader = FilesystemResourceLoader::locate();
        debug!("Created MagicLeapWindow");
        MagicLeapWindow {
            framebuffer_id,
            graphics_client,
            size: vec2i(max_width, max_height),
            frame_handle: ML_HANDLE_INVALID,
            virtual_camera_array,
            initial_camera_transform: None,
            resource_loader,
            pose_event: None,
            running: true,
            in_frame: false,
        }
    }

    pub fn size(&self) -> WindowSize {
        WindowSize {
            logical_size: self.size,
            backing_scale_factor: 1.0,
        }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn try_get_event(&mut self) -> Option<Event> {
        self.pose_event.take().map(Event::SetEyeTransforms)
    }

    fn begin_frame(&mut self) {
        if !self.in_frame {
            debug!("PF beginning frame");
            let mut params = unsafe { mem::zeroed() };
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
                MLGraphicsInitFrameParams(&mut params).unwrap();
                let mut result = MLGraphicsBeginFrame(self.graphics_client, &params, &mut self.frame_handle, &mut self.virtual_camera_array);
                if result == ML_RESULT_TIMEOUT {
                    info!("PF frame timeout");
                    let mut sleep = Duration::from_millis(1);
                    let max_sleep = Duration::from_secs(5);
                    while result == ML_RESULT_TIMEOUT {
                        sleep = (sleep * 2).min(max_sleep);
                        info!("PF exponential backoff {}ms", sleep.as_millis());
                        thread::sleep(sleep);
                        result = MLGraphicsBeginFrame(self.graphics_client, &params, &mut self.frame_handle, &mut self.virtual_camera_array);
                    }
                    info!("PF frame finished timeout");
                }
                result.unwrap();
            }
            let virtual_camera_array = &self.virtual_camera_array;
            let initial_camera = self.initial_camera_transform.get_or_insert_with(|| {
                let initial_offset = Transform4F::from_translation(0.0, 0.0, 1.0);
	        let mut camera = virtual_camera_array.virtual_cameras[0].transform;
		for i in 1..virtual_camera_array.num_virtual_cameras {
		    let next = virtual_camera_array.virtual_cameras[i as usize].transform;
		    camera = camera.lerp(next, 1.0 / (i as f32 + 1.0));
		}
		Transform4F::from(camera).post_mul(&initial_offset)
            });
            let camera_transforms = (0..virtual_camera_array.num_virtual_cameras)
                .map(|i| {
		    let camera = &virtual_camera_array.virtual_cameras[i as usize];
                    let projection = Transform4F::from(camera.projection);
                    let size = RectI::from(virtual_camera_array.viewport).size();
                    let perspective = Perspective::new(&projection, size);
                    let modelview_to_eye = Transform4F::from(camera.transform).inverse().post_mul(initial_camera);
                    OcularTransform { perspective, modelview_to_eye }
                })
                .collect();
            self.in_frame = true;
            self.pose_event = Some(camera_transforms);
            debug!("PF begun frame");
        }
    }

    fn end_frame(&mut self) {
        if self.in_frame {
            debug!("PF ending frame");
            unsafe {
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                for i in 0..self.virtual_camera_array.num_virtual_cameras {
                    let virtual_camera = &self.virtual_camera_array.virtual_cameras[i as usize];
                    MLGraphicsSignalSyncObjectGL(self.graphics_client, virtual_camera.sync_object).unwrap();
                }
                MLGraphicsEndFrame(self.graphics_client, self.frame_handle).unwrap();
            }
            self.in_frame = false;
            debug!("PF ended frame");
        }
    }
}

impl Drop for MagicLeapWindow {
    fn drop(&mut self) {
        self.end_frame();
        unsafe {
            gl::DeleteFramebuffers(1, &self.framebuffer_id);
            MLGraphicsDestroyClient(&mut self.graphics_client);
        }
    }
}

// Logging

pub struct MagicLeapLogger {
    tag: CString,
    level_filter: log::LevelFilter,
}

impl log::Log for MagicLeapLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level_filter
    }

    fn log(&self, record: &log::Record) {
        let lvl = match record.level() {
            log::Level::Error => MLLogLevel::Error,
            log::Level::Warn => MLLogLevel::Warning,
            log::Level::Info => MLLogLevel::Info,
            log::Level::Debug => MLLogLevel::Debug,
            log::Level::Trace => MLLogLevel::Verbose,
        };
        let mut msg = SmallVec::<[u8; 128]>::new();
        write!(msg, "{}\0", record.args()).unwrap();
        unsafe {
            MLLoggingLog(lvl, self.tag.as_ptr(), &msg[0] as *const _ as _);
        }
    }

    fn flush(&self) {}
}

impl MagicLeapLogger {
    pub fn new(tag: CString, level_filter: log::LevelFilter) -> Self {
        MagicLeapLogger { tag, level_filter }
    }
}

// Linear interpolation

impl MLVec3f {
    fn lerp(&self, other: MLVec3f, t: f32) -> MLVec3f {
        MLVec3f {
            x: util::lerp(self.x, other.x, t),
            y: util::lerp(self.y, other.y, t),
            z: util::lerp(self.z, other.z, t),
        }
    }
}

impl MLQuaternionf {
    fn lerp(&self, other: MLQuaternionf, t: f32) -> MLQuaternionf {
        MLQuaternionf {
            x: util::lerp(self.x, other.x, t),
            y: util::lerp(self.y, other.y, t),
            z: util::lerp(self.z, other.z, t),
            w: util::lerp(self.w, other.w, t),
        }
    }
}

impl MLTransform {
    fn lerp(&self, other: MLTransform, t: f32) -> MLTransform {
        MLTransform {
            rotation: self.rotation.lerp(other.rotation, t),
            position: self.position.lerp(other.position, t),
        }
    }
}

// Impl pathfinder traits for c-api types

impl From<MLTransform> for Transform4F {
    fn from(mat: MLTransform) -> Self {
        Transform4F::from(mat.rotation)
           .pre_mul(&Transform4F::from(mat.position))
    }
}

impl From<MLVec3f> for Transform4F {
    fn from(v: MLVec3f) -> Self {
        Transform4F::from_translation(v.x, v.y, v.z)
    }
}

impl From<MLRectf> for RectF {
    fn from(r: MLRectf) -> Self {
        RectF::new(Vector2F::new(r.x, r.y), Vector2F::new(r.w, r.h))
    }
}

impl From<MLRectf> for RectI {
    fn from(r: MLRectf) -> Self {
        RectF::from(r).to_i32()
    }
}

impl From<MLQuaternionf> for Transform4F {
    fn from(q: MLQuaternionf) -> Self {
        Transform4F::from_rotation_quaternion(F32x4::new(q.x, q.y, q.z, q.w))
    }
}

impl From<MLMat4f> for Transform4F {
    fn from(mat: MLMat4f) -> Self {
        let a = mat.matrix_colmajor;
        Transform4F::row_major(a[0], a[4], a[8],  a[12],
                                  a[1], a[5], a[9],  a[13],
                                  a[2], a[6], a[10], a[14],
                                  a[3], a[7], a[11], a[15])
    }
}

