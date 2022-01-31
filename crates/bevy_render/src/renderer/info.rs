use wgpu::{
    AdapterInfo, Backends, Features as WgpuFeatures, Limits as WgpuLimits, PowerPreference,
};

/// Provides information about the current renderer and the values it was configured with.
#[derive(Debug, Clone)]
pub struct RendererInfo {
    pub(crate) adapter_info: Option<AdapterInfo>,
    pub(crate) backends: Option<Backends>,
    pub(crate) power_preference: PowerPreference,
    pub(crate) features: WgpuFeatures,
    pub(crate) limits: WgpuLimits,
}

impl RendererInfo {
    /// Information about the graphics adapter in use by the current renderer.
    pub fn adapter_info(&self) -> Option<&AdapterInfo> {
        self.adapter_info.as_ref()
    }

    /// The backends the renderer could use, as configured with [`super::WgpuOptions`].
    pub fn backends(&self) -> Option<&Backends> {
        self.backends.as_ref()
    }

    /// The [`PowerPreference`] as configured with [`super::WgpuOptions`].
    pub fn power_preference(&self) -> &PowerPreference {
        &self.power_preference
    }

    /// The [`WgpuFeatures`] supported by the graphics adapter which the current renderer uses.
    pub fn features(&self) -> &WgpuFeatures {
        &self.features
    }

    /// The [`WgpuLimits`] supported by the graphics adapter which the current renderer uses.
    pub fn limits(&self) -> &WgpuLimits {
        &self.limits
    }
}
