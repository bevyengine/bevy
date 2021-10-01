use std::sync::atomic::{AtomicUsize, Ordering};
use std::{ops::Deref, sync::Arc};

static MAX_TEXTURE_ID: AtomicUsize = AtomicUsize::new(0);
static MAX_TEXTURE_VIEW_ID: AtomicUsize = AtomicUsize::new(0);
static MAX_SAMPLER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(usize);

impl TextureId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_TEXTURE_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureViewId(usize);

impl TextureViewId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_TEXTURE_VIEW_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(usize);

impl SamplerId {
    /// Creates a new id by incrementing the atomic id counter.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(MAX_SAMPLER_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub struct Texture {
    id: TextureId,
    value: Arc<wgpu::Texture>,
}

impl Texture {
    #[inline]
    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn create_view(&self, desc: &wgpu::TextureViewDescriptor) -> TextureView {
        TextureView::from(self.value.create_view(desc))
    }
}

impl From<wgpu::Texture> for Texture {
    fn from(value: wgpu::Texture) -> Self {
        Texture {
            id: TextureId::new(),
            value: Arc::new(value),
        }
    }
}

impl Deref for Texture {
    type Target = wgpu::Texture;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub enum TextureViewValue {
    TextureView(Arc<wgpu::TextureView>),
    SwapChainFrame(Arc<wgpu::SwapChainFrame>),
}

#[derive(Clone, Debug)]
pub struct TextureView {
    id: TextureViewId,
    value: TextureViewValue,
}

impl TextureView {
    #[inline]
    pub fn id(&self) -> TextureViewId {
        self.id
    }
}

impl From<wgpu::TextureView> for TextureView {
    fn from(value: wgpu::TextureView) -> Self {
        TextureView {
            id: TextureViewId::new(),
            value: TextureViewValue::TextureView(Arc::new(value)),
        }
    }
}

impl From<wgpu::SwapChainFrame> for TextureView {
    fn from(value: wgpu::SwapChainFrame) -> Self {
        TextureView {
            id: TextureViewId::new(),
            value: TextureViewValue::SwapChainFrame(Arc::new(value)),
        }
    }
}

impl Deref for TextureView {
    type Target = wgpu::TextureView;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match &self.value {
            TextureViewValue::TextureView(value) => value,
            TextureViewValue::SwapChainFrame(value) => &value.output.view,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sampler {
    id: SamplerId,
    value: Arc<wgpu::Sampler>,
}

impl Sampler {
    #[inline]
    pub fn id(&self) -> SamplerId {
        self.id
    }
}

impl From<wgpu::Sampler> for Sampler {
    fn from(value: wgpu::Sampler) -> Self {
        Sampler {
            id: SamplerId::new(),
            value: Arc::new(value),
        }
    }
}

impl Deref for Sampler {
    type Target = wgpu::Sampler;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub struct SwapChainFrame {
    id: TextureViewId,
    value: Arc<wgpu::SwapChainFrame>,
}

impl SwapChainFrame {
    #[inline]
    pub fn id(&self) -> TextureViewId {
        self.id
    }
}

impl From<wgpu::SwapChainFrame> for SwapChainFrame {
    fn from(value: wgpu::SwapChainFrame) -> Self {
        Self {
            id: TextureViewId::new(),
            value: Arc::new(value),
        }
    }
}

impl Deref for SwapChainFrame {
    type Target = wgpu::SwapChainFrame;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
