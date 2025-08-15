use crate::renderer::{
    RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
};
use alloc::borrow::Cow;

pub use wgpu::{
    Backends, Dx12Compiler, Features as WgpuFeatures, Gles3MinorVersion, InstanceFlags,
    Limits as WgpuLimits, MemoryHints, PowerPreference,
};
use wgpu::{DxcShaderModel, MemoryBudgetThresholds};

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

/// Provides configuration for renderer initialization. Use [`RenderDevice::features`](RenderDevice::features),
/// [`RenderDevice::limits`](RenderDevice::limits), and the [`RenderAdapterInfo`]
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
    /// Allows you to choose which minor version of GLES3 to use (3.0, 3.1, 3.2, or automatic)
    /// This only applies when using ANGLE and the GL backend.
    pub gles3_minor_version: Gles3MinorVersion,
    /// These are for controlling WGPU's debug information to eg. enable validation and shader debug info in release builds.
    pub instance_flags: InstanceFlags,
    /// This hints to the WGPU device about the preferred memory allocation strategy.
    pub memory_hints: MemoryHints,
    /// The thresholds for device memory budget.
    pub instance_memory_budget_thresholds: MemoryBudgetThresholds,
    /// If true, will force wgpu to use a software renderer, if available.
    pub force_fallback_adapter: bool,
    /// The name of the adapter to use.
    pub adapter_name: Option<String>,
}

impl Default for WgpuSettings {
    fn default() -> Self {
        let default_backends = if cfg!(all(
            feature = "webgl",
            target_arch = "wasm32",
            not(feature = "webgpu")
        )) {
            Backends::GL
        } else if cfg!(all(feature = "webgpu", target_arch = "wasm32")) {
            Backends::BROWSER_WEBGPU
        } else {
            Backends::all()
        };

        let backends = Some(Backends::from_env().unwrap_or(default_backends));

        let power_preference =
            PowerPreference::from_env().unwrap_or(PowerPreference::HighPerformance);

        let priority = settings_priority_from_env().unwrap_or(WgpuSettingsPriority::Functionality);

        let limits = if cfg!(all(
            feature = "webgl",
            target_arch = "wasm32",
            not(feature = "webgpu")
        )) || matches!(priority, WgpuSettingsPriority::WebGL2)
        {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            #[expect(clippy::allow_attributes, reason = "`unused_mut` is not always linted")]
            #[allow(
                unused_mut,
                reason = "This variable needs to be mutable if the `ci_limits` feature is enabled"
            )]
            let mut limits = wgpu::Limits::default();
            #[cfg(feature = "ci_limits")]
            {
                limits.max_storage_textures_per_shader_stage = 4;
                limits.max_texture_dimension_3d = 1024;
            }
            limits
        };

        let dx12_shader_compiler =
            Dx12Compiler::from_env().unwrap_or(if cfg!(feature = "statically-linked-dxc") {
                Dx12Compiler::StaticDxc
            } else {
                let dxc = "dxcompiler.dll";

                if cfg!(target_os = "windows") && std::fs::metadata(dxc).is_ok() {
                    Dx12Compiler::DynamicDxc {
                        dxc_path: String::from(dxc),
                        max_shader_model: DxcShaderModel::V6_7,
                    }
                } else {
                    Dx12Compiler::Fxc
                }
            });

        let gles3_minor_version = Gles3MinorVersion::from_env().unwrap_or_default();

        let instance_flags = InstanceFlags::default().with_env();

        Self {
            device_label: Default::default(),
            backends,
            power_preference,
            priority,
            features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            disabled_features: None,
            limits,
            constrained_limits: None,
            dx12_shader_compiler,
            gles3_minor_version,
            instance_flags,
            memory_hints: MemoryHints::default(),
            instance_memory_budget_thresholds: MemoryBudgetThresholds::default(),
            force_fallback_adapter: false,
            adapter_name: None,
        }
    }
}

#[derive(Clone)]
pub struct RenderResources(
    pub RenderDevice,
    pub RenderQueue,
    pub RenderAdapterInfo,
    pub RenderAdapter,
    pub RenderInstance,
    #[cfg(feature = "raw_vulkan_init")]
    pub  crate::renderer::raw_vulkan_init::AdditionalVulkanFeatures,
);

/// An enum describing how the renderer will initialize resources. This is used when creating the [`RenderPlugin`](crate::RenderPlugin).
#[expect(
    clippy::large_enum_variant,
    reason = "See https://github.com/bevyengine/bevy/issues/19220"
)]
pub enum RenderCreation {
    /// Allows renderer resource initialization to happen outside of the rendering plugin.
    Manual(RenderResources),
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
        #[cfg(feature = "raw_vulkan_init")]
        additional_vulkan_features: crate::renderer::raw_vulkan_init::AdditionalVulkanFeatures,
    ) -> Self {
        RenderResources(
            device,
            queue,
            adapter_info,
            adapter,
            instance,
            #[cfg(feature = "raw_vulkan_init")]
            additional_vulkan_features,
        )
        .into()
    }
}

impl From<RenderResources> for RenderCreation {
    fn from(value: RenderResources) -> Self {
        Self::Manual(value)
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
