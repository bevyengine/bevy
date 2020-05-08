// pathfinder/demo/magicleap/src/mocked_c_api.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A mocked Rust implementation of the Magic Leap C API, to allow it to build without the ML SDK

#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use crate::c_api::MLCoordinateFrameUID;
use crate::c_api::MLGraphicsClipExtentsInfoArray;
use crate::c_api::MLGraphicsFrameParams;
use crate::c_api::MLGraphicsOptions;
use crate::c_api::MLGraphicsRenderTargetsInfo;
use crate::c_api::MLGraphicsVirtualCameraInfoArray;
use crate::c_api::MLHandle;
use crate::c_api::MLHeadTrackingStaticData;
use crate::c_api::MLLogLevel;
use crate::c_api::MLResult;
use crate::c_api::MLSnapshotPtr;
use crate::c_api::MLTransform;
use std::os::raw::c_char;

pub unsafe fn MLGraphicsCreateClientGL(options: *const MLGraphicsOptions, gl_context: MLHandle, graphics_client : &mut MLHandle) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsDestroyClient(graphics_client: *mut MLHandle) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLHeadTrackingCreate(tracker: *mut MLHandle) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLHeadTrackingGetStaticData(head_tracker: MLHandle, data: *mut MLHeadTrackingStaticData) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLPerceptionGetSnapshot(snapshot: *mut MLSnapshotPtr) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLSnapshotGetTransform(snapshot: MLSnapshotPtr, id: *const MLCoordinateFrameUID, transform: *mut MLTransform) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLPerceptionReleaseSnapshot(snapshot: MLSnapshotPtr) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLLifecycleSetReadyIndication() -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsGetClipExtents(graphics_client: MLHandle, array: *mut MLGraphicsClipExtentsInfoArray) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsGetRenderTargets(graphics_client: MLHandle, targets: *mut MLGraphicsRenderTargetsInfo) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsInitFrameParams(params: *mut MLGraphicsFrameParams) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsBeginFrame(graphics_client: MLHandle, params: *const MLGraphicsFrameParams, frame_handle: *mut MLHandle, virtual_camera_array: *mut MLGraphicsVirtualCameraInfoArray) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsEndFrame(graphics_client: MLHandle, frame_handle: MLHandle) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGraphicsSignalSyncObjectGL(graphics_client: MLHandle, sync_object: MLHandle) -> MLResult {
    unimplemented!()
}

pub unsafe fn MLGetResultString(result_code: MLResult) -> *const c_char {
    unimplemented!()
}

pub unsafe fn MLLoggingLogLevelIsEnabled(lvl: MLLogLevel) -> bool {
    unimplemented!()
}

pub unsafe fn MLLoggingLog(lvl: MLLogLevel, tag: *const c_char, message: *const c_char) {
    unimplemented!()
}

