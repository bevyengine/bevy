use std::borrow::Cow;

pub use wgpu::{Backends, Features, Limits, PowerPreference};

#[derive(Clone)]
pub struct WgpuOptions {
    pub device_label: Option<Cow<'static, str>>,
    pub backends: Backends,
    pub power_preference: PowerPreference,
    pub features: Features,
    pub limits: Limits,
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
            wgpu::Limits::default()
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
