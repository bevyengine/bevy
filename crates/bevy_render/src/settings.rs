use crate::renderer::{
    RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
};
use std::borrow::Cow;

pub use wgpu::{
    Backends, Dx12Compiler, Features as WgpuFeatures, Limits as WgpuLimits, PowerPreference,
};

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
/// [`RenderDevice::limits`](crate::renderer::RenderDevice::limits), and the [`RenderAdapterInfo`](crate::renderer::RenderAdapterInfo)
/// resource to get runtime information about the actual adapter, backend, features, and limits.
/// NOTE: [`Backends::DX12`](Backends::DX12), [`Backends::METAL`](Backends::METAL), and
/// [`Backends::VULKAN`](Backends::VULKAN) are enabled by default for non-web and the best choice
/// is automatically selected. Web using the `webgl` feature uses [`Backends::GL`](Backends::GL).
/// NOTE: If you want to use [`Backends::GL`](Backends::GL) in a native app on `Windows` and/or `macOS`, you must
/// use [`ANGLE`](https://github.com/gfx-rs/wgpu#angle). This is because wgpu requires EGL to
/// create a GL context without a window and only ANGLE supports that.
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
    /// The shader compiler to use for the DX12 backend.
    pub dx12_shader_compiler: Dx12Compiler,
}

impl Default for WgpuSettings {
    fn default() -> Self {
        let default_backends = if cfg!(all(feature = "webgl", target_arch = "wasm32")) {
            Backends::GL
        } else {
            Backends::all()
        };

        let backends = Some(wgpu::util::backend_bits_from_env().unwrap_or(default_backends));

        let power_preference =
            wgpu::util::power_preference_from_env().unwrap_or(PowerPreference::HighPerformance);

        let priority = settings_priority_from_env().unwrap_or(WgpuSettingsPriority::Functionality);

        let limits = if cfg!(all(feature = "webgl", target_arch = "wasm32"))
            || matches!(priority, WgpuSettingsPriority::WebGL2)
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
            features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
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
    Manual(
        RenderDevice,
        RenderQueue,
        RenderAdapterInfo,
        RenderAdapter,
        RenderInstance,
    ),
    /// Lets the rendering plugin create resources itself.
    Automatic(WgpuSettings),
}

impl RenderCreation {
    /// Function to create a [`RenderCreation::Manual`] variant.
    pub fn manual(
        device: RenderDevice,
        queue: RenderQueue,
        adapter_info: RenderAdapterInfo,
        adapter: RenderAdapter,
        instance: RenderInstance,
    ) -> Self {
        Self::Manual(device, queue, adapter_info, adapter, instance)
    }
}

impl Default for RenderCreation {
    fn default() -> Self {
        Self::Automatic(Default::default())
    }
}

impl From<WgpuSettings> for RenderCreation {
    fn from(value: WgpuSettings) -> Self {
        Self::Automatic(value)
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
