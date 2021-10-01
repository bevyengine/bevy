use crate::render_resource::{next_id, Counter, Id};
use std::{ops::Deref, sync::Arc};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Id);

impl TextureId {
    /// Creates a new, unique [`TextureId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: TextureId::new().expect("The system ran out of unique `TextureId`s."),
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

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureViewId(Id);

impl TextureViewId {
    /// Creates a new, unique [`TextureViewId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: TextureViewId::new().expect("The system ran out of unique `TextureViewId`s."),
            value: TextureViewValue::TextureView(Arc::new(value)),
        }
    }
}

impl From<wgpu::SwapChainFrame> for TextureView {
    fn from(value: wgpu::SwapChainFrame) -> Self {
        TextureView {
            id: TextureViewId::new().expect("The system ran out of unique `TextureViewId`s."),
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

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(Id);

impl SamplerId {
    /// Creates a new, unique [`SamplerId`].
    /// Returns [`None`] if the supply of unique ids has been exhausted.
    fn new() -> Option<Self> {
        static COUNTER: Counter = Counter::new(0);
        next_id(&COUNTER).map(Self)
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
            id: SamplerId::new().expect("The system ran out of unique `SamplerId`s."),
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
            id: TextureViewId::new().expect("The system ran out of unique `TextureViewId`s."),
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
