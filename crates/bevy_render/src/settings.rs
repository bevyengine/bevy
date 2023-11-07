use crate::{
    gpu_resource::*,
    renderer::{GpuAdapter, GpuAdapterInfo, GpuDevice, GpuInstance, GpuQueue},
};
use std::borrow::Cow;

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
/// [`GpuDevice::limits`](crate::renderer::GpuDevice::limits), and the [`GpuAdapterInfo`]
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
    pub backends: Option<GpuBackends>,
    pub power_preference: GpuPowerPreference,
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
    /// The shader compiler to use for the DX12 backend.
    pub dx12_shader_compiler: Dx12Compiler,
}

impl Default for GpuSettings {
    fn default() -> Self {
        let default_backends = if cfg!(all(feature = "webgl", target_arch = "wasm32")) {
            GpuBackends::GL
        } else {
            GpuBackends::all()
        };

        let backends = Some(wgpu::util::backend_bits_from_env().unwrap_or(default_backends));

        let power_preference =
            wgpu::util::power_preference_from_env().unwrap_or(GpuPowerPreference::HighPerformance);

        let priority = settings_priority_from_env().unwrap_or(GpuSettingsPriority::Functionality);

        let limits = if cfg!(all(feature = "webgl", target_arch = "wasm32"))
            || matches!(priority, GpuSettingsPriority::WebGL2)
        {
            GpuLimits::downlevel_webgl2_defaults()
        } else {
            #[allow(unused_mut)]
            let mut limits = GpuLimits::default();
            #[cfg(feature = "ci_limits")]
            {
                limits.max_storage_textures_per_shader_stage = 4;
                limits.max_texture_dimension_3d = 1024;
            }
            limits
        };

        let dx12_compiler =
            wgpu::util::dx12_shader_compiler_from_env().unwrap_or(Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            });

        Self {
            device_label: Default::default(),
            backends,
            power_preference,
            priority,
            features: GpuFeatures::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            disabled_features: None,
            limits,
            constrained_limits: None,
            dx12_shader_compiler: dx12_compiler,
        }
    }
}

/// An enum describing how the renderer will initialize resources. This is used when creating the [`RenderPlugin`](crate::RenderPlugin).
pub enum RenderCreation {
    /// Allows renderer resource initialization to happen outside of the rendering plugin.
    Manual(GpuDevice, GpuQueue, GpuAdapterInfo, GpuAdapter, GpuInstance),
    /// Lets the rendering plugin create resources itself.
    Automatic(GpuSettings),
}

impl RenderCreation {
    /// Function to create a [`RenderCreation::Manual`] variant.
    pub fn manual(
        gpu_device: GpuDevice,
        gpu_queue: GpuQueue,
        gpu_adapter_info: GpuAdapterInfo,
        gpu_adapter: GpuAdapter,
        gpu_instance: GpuInstance,
    ) -> Self {
        Self::Manual(
            gpu_device,
            gpu_queue,
            gpu_adapter_info,
            gpu_adapter,
            gpu_instance,
        )
    }
}

impl Default for RenderCreation {
    fn default() -> Self {
        Self::Automatic(Default::default())
    }
}

impl From<GpuSettings> for RenderCreation {
    fn from(value: GpuSettings) -> Self {
        Self::Automatic(value)
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
