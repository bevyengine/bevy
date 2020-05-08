// pathfinder/demo/magicleap/src/c_api.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Bindings to the C MagicLeap API

#![allow(dead_code)]

use gl::types::GLuint;
use std::error::Error;
use std::ffi::CStr;
use std::fmt;
#[cfg(not(feature = "mocked"))]
use std::os::raw::c_char;
use std::os::raw::c_void;

// Types from the MagicLeap C API

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MLHandle(u64);

impl MLHandle {
    pub fn as_gl_uint(self) -> GLuint {
        self.0 as GLuint
    }
}

impl<T> From<*mut T> for MLHandle {
    fn from(ptr: *mut T) -> MLHandle {
        MLHandle(ptr as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct MLResult(u32);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsOptions {
    pub graphics_flags: u32,
    pub color_format: MLSurfaceFormat,
    pub depth_format: MLSurfaceFormat,
}

impl Default for MLGraphicsOptions {
    fn default() -> MLGraphicsOptions {
        MLGraphicsOptions {
            graphics_flags: 0,
            color_format: MLSurfaceFormat::RGBA8UNormSRGB,
            depth_format: MLSurfaceFormat::D32Float,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsRenderTargetsInfo {
    pub min_clip: f32,
    pub max_clip: f32,
    pub num_virtual_cameras: u32,
    pub buffers: [MLGraphicsRenderBufferInfo; ML_BUFFER_COUNT],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsRenderBufferInfo {
    pub color: MLGraphicsRenderTarget,
    pub depth: MLGraphicsRenderTarget,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsRenderTarget {
    pub width: u32,
    pub height: u32,
    pub id: MLHandle,
    pub format: MLSurfaceFormat,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum MLSurfaceFormat {
    Unknown = 0,
    RGBA8UNorm,
    RGBA8UNormSRGB,
    RGB10A2UNorm,
    RGBA16Float,
    D32Float,
    D24NormS8,
    D32FloatS8,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsVirtualCameraInfoArray {
    pub num_virtual_cameras: u32,
    pub color_id: MLHandle,
    pub depth_id: MLHandle,
    pub viewport: MLRectf,
    pub virtual_cameras: [MLGraphicsVirtualCameraInfo; ML_VIRTUAL_CAMERA_COUNT],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsVirtualCameraInfo {
    pub left_half_angle: f32,
    pub right_half_angle: f32,
    pub top_half_angle: f32,
    pub bottom_half_angle: f32,
    pub sync_object: MLHandle,
    pub projection: MLMat4f,
    pub transform: MLTransform,
    pub virtual_camera_name: MLGraphicsVirtualCameraName,
}

#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum MLGraphicsVirtualCameraName {
    Combined = -1,
    Left = 0,
    Right,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsFrameParams {
    pub near_clip: f32,
    pub far_clip: f32,
    pub focus_distance: f32,
    pub surface_scale: f32,
    pub protected_surface: bool,
    pub projection_type: MLGraphicsProjectionType,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLSnapshotPtr(*mut c_void);

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLCoordinateFrameUID {
    pub data: [u64; 2],
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLHeadTrackingStaticData {
    pub coord_frame_head: MLCoordinateFrameUID,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsClipExtentsInfo {
    pub virtual_camera_name: MLGraphicsVirtualCameraName,
    pub projection: MLMat4f,
    pub transform: MLTransform,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLGraphicsClipExtentsInfoArray {
    pub num_virtual_cameras: u32,
    pub full_extents: MLGraphicsClipExtentsInfo,
    pub virtual_camera_extents: [MLGraphicsClipExtentsInfo; ML_VIRTUAL_CAMERA_COUNT],
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum MLGraphicsProjectionType {
    SignedZ = 0,
    ReversedInfiniteZ = 1,
    UnsignedZ = 2,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLTransform {
    pub rotation: MLQuaternionf,
    pub position: MLVec3f,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLVec3f {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLRectf {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLQuaternionf {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MLMat4f {
    pub matrix_colmajor: [f32; 16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MLLogLevel {
    Fatal = 0,
    Error = 1,
    Warning = 2,
    Info = 3,
    Debug = 4,
    Verbose = 5,
}

// Constants from the MagicLeap C API

pub const ML_RESULT_OK: MLResult = MLResult(0);
pub const ML_RESULT_TIMEOUT: MLResult = MLResult(2);
pub const ML_RESULT_UNSPECIFIED_FAILURE: MLResult = MLResult(4);
pub const ML_HANDLE_INVALID: MLHandle = MLHandle(0xFFFFFFFFFFFFFFFF);
pub const ML_BUFFER_COUNT: usize = 3;
pub const ML_VIRTUAL_CAMERA_COUNT: usize = 2;

// ML error handling

impl fmt::Display for MLResult {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let cmessage = unsafe { CStr::from_ptr(MLGetResultString(*self)) };
        let message = cmessage.to_str().or(Err(fmt::Error))?;
        formatter.write_str(message)
    }
}

impl MLResult {
    pub fn ok(self) -> Result<(), MLResult> {
        if self == ML_RESULT_OK {
            Ok(())
        } else {
            Err(self)
        }
    }

    pub fn unwrap(self) {
        self.ok().unwrap()
    }
}

impl Error for MLResult {
}

// Functions from the MagicLeap C API

#[cfg(not(feature = "mocked"))]
extern "C" {
    pub fn MLGraphicsCreateClientGL(options: *const MLGraphicsOptions, gl_context: MLHandle, graphics_client : &mut MLHandle) -> MLResult;
    pub fn MLGraphicsDestroyClient(graphics_client: *mut MLHandle) -> MLResult;
    pub fn MLHeadTrackingCreate(tracker: *mut MLHandle) -> MLResult;
    pub fn MLHeadTrackingGetStaticData(head_tracker: MLHandle, data: *mut MLHeadTrackingStaticData) -> MLResult;
    pub fn MLPerceptionGetSnapshot(snapshot: *mut MLSnapshotPtr) -> MLResult;
    pub fn MLSnapshotGetTransform(snapshot: MLSnapshotPtr, id: *const MLCoordinateFrameUID, transform: *mut MLTransform) -> MLResult;
    pub fn MLPerceptionReleaseSnapshot(snapshot: MLSnapshotPtr) -> MLResult;
    pub fn MLLifecycleSetReadyIndication() -> MLResult;
    pub fn MLGraphicsGetClipExtents(graphics_client: MLHandle, array: *mut MLGraphicsClipExtentsInfoArray) -> MLResult;
    pub fn MLGraphicsGetRenderTargets(graphics_client: MLHandle, targets: *mut MLGraphicsRenderTargetsInfo) -> MLResult;
    pub fn MLGraphicsInitFrameParams(params: *mut MLGraphicsFrameParams) -> MLResult;
    pub fn MLGraphicsBeginFrame(graphics_client: MLHandle, params: *const MLGraphicsFrameParams, frame_handle: *mut MLHandle, virtual_camera_array: *mut MLGraphicsVirtualCameraInfoArray) -> MLResult;
    pub fn MLGraphicsEndFrame(graphics_client: MLHandle, frame_handle: MLHandle) -> MLResult;
    pub fn MLGraphicsSignalSyncObjectGL(graphics_client: MLHandle, sync_object: MLHandle) -> MLResult;
    pub fn MLGetResultString(result_code: MLResult) -> *const c_char;
    pub fn MLLoggingLogLevelIsEnabled(lvl: MLLogLevel) -> bool;
    pub fn MLLoggingLog(lvl: MLLogLevel, tag: *const c_char, message: *const c_char);
}

#[cfg(feature = "mocked")]
pub use crate::mocked_c_api::*;
