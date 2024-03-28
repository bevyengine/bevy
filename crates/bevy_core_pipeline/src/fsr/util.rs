use bevy_render::{
    render_resource::hal::vulkan::{Device, VulkanApi},
    renderer::RenderDevice,
};
use fsr::{FfxErrorCode, FFX_OK};

pub fn call_hal<T>(
    render_device: &RenderDevice,
    f: impl FnOnce(&Device) -> Option<T>,
) -> Option<T> {
    let wgpu_device = render_device.wgpu_device();
    unsafe {
        wgpu_device.as_hal::<VulkanApi, _, _>(|device| {
            (f)(device.expect("FsrPlugin can only be used on the Vulkan graphics backend"))
        })
    }
    .flatten()
}

// TODO: Proper Result type
pub(crate) fn ffx_check_result(result: FfxErrorCode) -> Option<()> {
    if result == FFX_OK {
        Some(())
    } else {
        None
    }
}
