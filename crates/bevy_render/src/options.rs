use std::borrow::Cow;

pub use wgpu::{Backends, Features as WgpuFeatures, Limits as WgpuLimits, PowerPreference};

#[derive(Clone)]
pub struct WgpuOptions {
    pub device_label: Option<Cow<'static, str>>,
    pub backends: Backends,
    pub power_preference: PowerPreference,
    pub features: WgpuFeatures,
    pub limits: WgpuLimits,
}

impl Default for WgpuOptions {
    fn default() -> Self {
        let default_backends = if cfg!(target_arch = "wasm32") {
            Backends::GL
        } else {
            Backends::PRIMARY
        };

        let backends = wgpu::util::backend_bits_from_env().unwrap_or(default_backends);

        let limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            #[allow(unused_mut)]
            let mut limits = wgpu::Limits::default();
            #[cfg(feature = "ci_limits")]
            {
                limits.max_storage_textures_per_shader_stage = 4;
                limits.max_texture_dimension_3d = 1024;
            }
            limits
        };

        Self {
            device_label: Default::default(),
            backends,
            power_preference: PowerPreference::HighPerformance,
            features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits,
        }
    }
}
