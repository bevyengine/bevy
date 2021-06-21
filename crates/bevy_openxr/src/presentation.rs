use std::ffi::c_void;

use crate::{FrameStream, OpenXrContext, SessionBackend};
use bevy_app::Plugin;
use bevy_xr::presentation::{
    GraphicsContextHandles, VkGetInstanceProcAddr, XrPresentationError,
    XrPresentationContext, XrSessionHandle,
};
use openxr as xr;

impl XrPresentationContext for OpenXrContext {
    unsafe fn create_vulkan_instance(
        &self,
        get_instance_proc_addr: VkGetInstanceProcAddr,
        instance_create_info: *const c_void,
    ) -> Result<*const c_void, XrPresentationError> {
        if self.instance.exts().khr_vulkan_enable2.is_none() {
            return Err(XrPresentationError(
                "The active OpenXR runtime does not support khr_vulkan_enable2".into(),
            ));
        }

        Ok(self
            .instance
            .create_vulkan_instance(self.system_id, get_instance_proc_addr, instance_create_info)
            .unwrap()
            .unwrap())
    }

    unsafe fn vulkan_graphics_device(&self, instance: *const c_void) -> *const c_void {
        self.instance
            .vulkan_graphics_device(self.system_id, instance)
            .unwrap()
    }

    unsafe fn create_vulkan_device(
        &self,
        get_instance_proc_addr: VkGetInstanceProcAddr,
        physical_device: *const c_void,
        device_create_info: *const c_void,
    ) -> *const c_void {
        self.instance
            .create_vulkan_device(
                self.system_id,
                get_instance_proc_addr,
                physical_device,
                device_create_info,
            )
            .unwrap()
            .unwrap()
    }

    unsafe fn initialize_session_from_graphics_handles(
        &self,
        context_handles: GraphicsContextHandles,
    ) -> Result<Box<dyn XrSessionHandle>, XrPresentationError> {
        let (session_backend, frame_waiter, frame_stream) = match context_handles {
            GraphicsContextHandles::Vulkan {
                instance,
                physical_device,
                device,
                queue_family_index,
                queue_index,
            } => {
                let (session, frame_waiter, frame_stream) = self
                    .instance
                    .create_session(
                        self.system_id,
                        &xr::vulkan::SessionCreateInfo {
                            instance,
                            physical_device,
                            device,
                            queue_family_index,
                            queue_index,
                        },
                    )
                    .unwrap();
                (
                    SessionBackend::Vulkan(session),
                    frame_waiter,
                    FrameStream::Vulkan(frame_stream),
                )
            }
            #[cfg(windows)]
            GraphicsContextHandles::D3D11 { device } => {
                let (session, frame_waiter, frame_stream) = self
                    .instance
                    .create_session(
                        self.system_id,
                        &xr::d3d::SessionCreateInfo {
                            device: device as _,
                        },
                    )
                    .unwrap();
                (
                    SessionBackend::D3D11(session),
                    frame_waiter,
                    FrameStream::D3D11(frame_stream),
                )
            }
            _ => return Err(XrPresentationError("Unsupported backend".into())),
        };

        todo!()
    }
}