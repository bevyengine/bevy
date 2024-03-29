use bevy_render::{
    render_resource::{
        hal::vulkan::{Device, VulkanApi},
        TextureFormat, TextureUsages,
    },
    renderer::RenderDevice,
    texture::CachedTexture,
};
use fsr::{
    ffxGetTextureResourceVK, FfxErrorCode, FfxFsr2Context, FfxResource,
    FfxResourceStates_FFX_RESOURCE_STATE_COMPUTE_READ, VkFormat, VkImage, VkImageView, FFX_OK,
};
use std::ptr;

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

pub fn ffx_get_texture(texture: &CachedTexture, context: &mut FfxFsr2Context) -> FfxResource {
    unsafe {
        ffxGetTextureResourceVK(
            context,
            todo!(),
            todo!(),
            texture.texture.width(),
            texture.texture.height(),
            match texture.texture.format() {
                TextureFormat::Rgba8UnormSrgb => VkFormat::R8G8B8A8_SRGB,
                TextureFormat::Rgba16Float => VkFormat::R16G16B16A16_SFLOAT,
                TextureFormat::Depth32Float => VkFormat::D32_SFLOAT,
                TextureFormat::Rg16Float => VkFormat::R16G16_SFLOAT,
                _ => unreachable!("Invalid FSR texture format"),
            },
            ptr::null_mut(),
            FfxResourceStates_FFX_RESOURCE_STATE_COMPUTE_READ,
        )
    }
}

pub fn ffx_null_texture(context: &mut FfxFsr2Context) -> FfxResource {
    unsafe {
        ffxGetTextureResourceVK(
            context,
            VkImage::null(),
            VkImageView::null(),
            1,
            1,
            VkFormat::UNDEFINED,
            ptr::null_mut(),
            FfxResourceStates_FFX_RESOURCE_STATE_COMPUTE_READ,
        )
    }
}

// TODO: Proper Result type
pub(crate) fn ffx_check_result(result: FfxErrorCode) -> Option<()> {
    if result == FFX_OK {
        Some(())
    } else {
        None
    }
}
