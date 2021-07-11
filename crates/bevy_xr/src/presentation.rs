use std::sync::Arc;

pub enum XrVisibilityState {
    Hidden,
    Visible,
    Focused,
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
