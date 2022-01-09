use std::borrow::Cow;

pub use wgpu::{Backends, Features as WgpuFeatures, Limits as WgpuLimits, PowerPreference};

#[derive(Clone)]
pub enum WgpuOptionsPriority {
    Compatibility,
    Functionality,
    WebGL2,
}

#[derive(Clone)]
pub struct WgpuOptions {
    pub device_label: Option<Cow<'static, str>>,
    pub backends: Option<Backends>,
    pub power_preference: PowerPreference,
    pub priority: WgpuOptionsPriority,
    pub features: WgpuFeatures,
    pub limits: WgpuLimits,
}

impl Default for WgpuOptions {
    fn default() -> Self {
        let default_backends = if cfg!(feature = "webgl") {
            Backends::GL
        } else {
            Backends::PRIMARY
        };

        let backends = Some(wgpu::util::backend_bits_from_env().unwrap_or(default_backends));

        let priority = options_priority_from_env().unwrap_or(WgpuOptionsPriority::Functionality);

        let limits = if cfg!(feature = "webgl") || matches!(priority, WgpuOptionsPriority::WebGL2) {
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
            priority,
            features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            limits,
        }
    }
}

/// Get a features/limits priority from the environment variable WGPU_OPTIONS_PRIO
pub fn options_priority_from_env() -> Option<WgpuOptionsPriority> {
    Some(
        match std::env::var("WGPU_OPTIONS_PRIO")
            .as_deref()
            .map(str::to_lowercase)
            .as_deref()
        {
            Ok("compatibility") => WgpuOptionsPriority::Compatibility,
            Ok("functionality") => WgpuOptionsPriority::Functionality,
            Ok("webgl2") => WgpuOptionsPriority::WebGL2,
            _ => return None,
        },
    )
}
