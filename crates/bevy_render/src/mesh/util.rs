use wgpu::TextureFormat;

// PERF: vulkan docs recommend using 24 bit depth for better performance
pub const CORE_3D_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

/// On WebGL and WebGPU, we must disable irradiance volumes, as otherwise we can
/// overflow the number of texture bindings when deferred rendering is in use
/// (see issue #11885).
pub const IRRADIANCE_VOLUMES_ARE_USABLE: bool = cfg!(not(target_arch = "wasm32"));

pub const TONEMAPPING_LUT_TEXTURE_BINDING_INDEX: u32 = 18;
pub const TONEMAPPING_LUT_SAMPLER_BINDING_INDEX: u32 = 19;
