use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrEnvironmentBlendMode {
    Opaque,
    AlphaBlend,
    Additive,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrInteractionMode {
    ScreenSpace,
    WorldSpace,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrVisibilityState {
    VisibleFocused,
    VisibleUnfocused,
    Hidden,
}

pub struct XrGraphicsContext {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
}

// Trait implemented by XR backends that support display mode.
pub trait XrPresentationSession: Send + Sync + 'static {
    fn get_swapchains(&mut self) -> Vec<Vec<u64>>;
}
