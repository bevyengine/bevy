use bevy_ecs::prelude::World;
use std::{ffi::c_void, os::raw::c_char, ptr};

pub struct XrPresentationError(pub String);

pub type VkGetInstanceProcAddr =
    unsafe extern "system" fn(*const c_void, *const c_char) -> Option<unsafe extern "system" fn()>;

// This is a copy of openxrs SessionCreateInfo structures
pub enum GraphicsContextHandles {
    Vulkan {
        instance: *const c_void,
        physical_device: *const c_void,
        device: *const c_void,
        queue_family_index: u32,
        queue_index: u32,
    },
    D3D11 {
        device: *mut c_void,
    },
    WebGpu {
        // todo
    },
}

// Manages the lifetime of the XR session when not in TrackingOnly mode. Backends must implement
// Drop for destroying the inner session
pub trait XrSessionHandle {
    fn get_swapchains(&mut self) -> Result<Vec<Vec<u64>>, XrPresentationError>;
}

// Trait implemented by XR backends that support display mode.
pub trait XrPresentationContext: Send + Sync + 'static {
    /// Note: this is OpenXR-Vulkan specific. Any other XR backend should return an error
    /// # Safety
    /// Arguments must be valid Vulkan pointers
    unsafe fn create_vulkan_instance(
        &self,
        _get_instance_proc_addr: VkGetInstanceProcAddr,
        _instance_create_info: *const c_void,
    ) -> Result<*const c_void, XrPresentationError> {
        Err(XrPresentationError("Method not supported".into()))
    }

    /// Note: this is OpenXR-Vulkan specific.
    /// # Safety
    /// Arguments must be valid Vulkan pointers
    unsafe fn vulkan_graphics_device(&self, _instance: *const c_void) -> *const c_void {
        ptr::null()
    }

    /// Note: this is OpenXR-Vulkan specific.
    /// # Safety
    /// Arguments must be valid Vulkan pointers
    unsafe fn create_vulkan_device(
        &self,
        _get_instance_proc_addr: VkGetInstanceProcAddr,
        _physical_device: *const c_void,
        _device_create_info: *const c_void,
    ) -> *const c_void {
        ptr::null()
    }

    /// # Safety
    /// The returned handle must be dropped before destroying the graphics instance and device
    unsafe fn initialize_session_from_graphics_handles(
        &self,
        world: &mut World,
        context_handles: GraphicsContextHandles,
    ) -> Result<Box<dyn XrSessionHandle>, XrPresentationError>;
}
