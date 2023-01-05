use std::borrow::Cow;

pub use wgpu::{Backends, Features as GpuFeatures, Limits as GpuLimits, PowerPreference};

/// Configures the priority used when automatically configuring the features/limits of `wgpu`.
#[derive(Clone)]
pub enum GpuSettingsPriority {
    /// WebGPU default features and limits
    Compatibility,
    /// The maximum supported features and limits of the adapter and backend
    Functionality,
    /// WebGPU default limits plus additional constraints in order to be compatible with WebGL2
    WebGL2,
}

/// Provides configuration for renderer initialization. Use [`GpuDevice::features`](crate::renderer::GpuDevice::features),
/// [`GpuDevice::limits`](crate::renderer::GpuDevice::limits), and the [`GpuAdapterInfo`](crate::renderer::GpuAdapterInfo)
/// resource to get runtime information about the actual adapter, backend, features, and limits.
/// NOTE: [`Backends::DX12`](Backends::DX12), [`Backends::METAL`](Backends::METAL), and
/// [`Backends::VULKAN`](Backends::VULKAN) are enabled by default for non-web and the best choice
/// is automatically selected. Web using the `webgl` feature uses [`Backends::GL`](Backends::GL).
/// NOTE: If you want to use [`Backends::GL`](Backends::GL) in a native app on `Windows` and/or `macOS`, you must
/// use [`ANGLE`](https://github.com/gfx-rs/wgpu#angle). This is because wgpu requires EGL to
/// create a GL context without a window and only ANGLE supports that.
#[derive(Clone)]
pub struct GpuSettings {
    pub device_label: Option<Cow<'static, str>>,
    pub backends: Option<Backends>,
    pub power_preference: PowerPreference,
    pub priority: GpuSettingsPriority,
    /// The features to ensure are enabled regardless of what the adapter/backend supports.
    /// Setting these explicitly may cause renderer initialization to fail.
    pub features: GpuFeatures,
    /// The features to ensure are disabled regardless of what the adapter/backend supports
    pub disabled_features: Option<GpuFeatures>,
    /// The imposed limits.
    pub limits: GpuLimits,
    /// The constraints on limits allowed regardless of what the adapter/backend supports
    pub constrained_limits: Option<GpuLimits>,
}

impl Default for GpuSettings {
    fn default() -> Self {
        let default_backends = if cfg!(feature = "webgl") {
            Backends::GL
        } else {
            Backends::PRIMARY
        };

        let backends = Some(wgpu::util::backend_bits_from_env().unwrap_or(default_backends));

        let priority = settings_priority_from_env().unwrap_or(GpuSettingsPriority::Functionality);

        let limits = if cfg!(feature = "webgl") || matches!(priority, GpuSettingsPriority::WebGL2) {
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
pub fn settings_priority_from_env() -> Option<GpuSettingsPriority> {
    Some(
        match std::env::var("WGPU_SETTINGS_PRIO")
            .as_deref()
            .map(str::to_lowercase)
            .as_deref()
        {
            Ok("compatibility") => GpuSettingsPriority::Compatibility,
            Ok("functionality") => GpuSettingsPriority::Functionality,
            Ok("webgl2") => GpuSettingsPriority::WebGL2,
            _ => return None,
        },
    )
}
