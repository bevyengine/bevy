use std::borrow::Cow;

pub use wgpu::{Backends, Features as WgpuFeatures, Limits as WgpuLimits, PowerPreference};

/// Configures the priority used when automatically configuring the features/limits of `wgpu`.
#[derive(Clone)]
pub enum WgpuSettingsPriority {
    /// WebGPU default features and limits
    Compatibility,
    /// The maximum supported features and limits of the adapter and backend
    Functionality,
    /// WebGPU default limits plus additional constraints in order to be compatible with WebGL2
    WebGL2,
}

/// Provides configuration for renderer initialization. Use [`RenderDevice::features`](crate::renderer::RenderDevice::features),
/// [`RenderDevice::limits`](crate::renderer::RenderDevice::limits), and the [`WgpuAdapterInfo`](crate::render_resource::WgpuAdapterInfo)
/// resource to get runtime information about the actual adapter, backend, features, and limits.
#[derive(Clone)]
pub struct WgpuSettings {
    pub device_label: Option<Cow<'static, str>>,
    pub backends: Option<Backends>,
    pub power_preference: PowerPreference,
    pub priority: WgpuSettingsPriority,
    /// The features to ensure are enabled regardless of what the adapter/backend supports.
    /// Setting these explicitly may cause renderer initialization to fail.
    pub features: WgpuFeatures,
    /// The features to ensure are disabled regardless of what the adapter/backend supports
    pub disabled_features: Option<WgpuFeatures>,
    /// The imposed limits.
    pub limits: WgpuLimits,
    /// The constraints on limits allowed regardless of what the adapter/backend supports
    pub constrained_limits: Option<WgpuLimits>,
}

impl Default for WgpuSettings {
    fn default() -> Self {
        let default_backends = if cfg!(feature = "webgl") {
            Backends::GL
        } else {
            Backends::PRIMARY
        };

        let backends = Some(wgpu::util::backend_bits_from_env().unwrap_or(default_backends));

        let priority = settings_priority_from_env().unwrap_or(WgpuSettingsPriority::Functionality);

        let limits = if cfg!(feature = "webgl") || matches!(priority, WgpuSettingsPriority::WebGL2)
        {
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
            disabled_features: None,
            limits,
            constrained_limits: None,
        }
    }
}

/// Get a features/limits priority from the environment variable `WGPU_SETTINGS_PRIO`
pub fn settings_priority_from_env() -> Option<WgpuSettingsPriority> {
    Some(
        match std::env::var("WGPU_SETTINGS_PRIO")
            .as_deref()
            .map(str::to_lowercase)
            .as_deref()
        {
            Ok("compatibility") => WgpuSettingsPriority::Compatibility,
            Ok("functionality") => WgpuSettingsPriority::Functionality,
            Ok("webgl2") => WgpuSettingsPriority::WebGL2,
            _ => return None,
        },
    )
}
