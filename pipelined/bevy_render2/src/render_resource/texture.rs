use bevy_utils::Uuid;
use std::{ops::Deref, sync::Arc};

/// A [`Texture`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Uuid);

/// A GPU-accessible texture.
///
/// May be converted from and dereferences to a wgpu [`Texture`](wgpu::Texture).
/// Can be created via [`RenderDevice::create_texture`](crate::renderer::render_device::RenderDevice::create_texture).
#[derive(Clone, Debug)]
pub struct Texture {
    id: TextureId,
    value: Arc<wgpu::Texture>,
}

impl Texture {
    /// Returns the [`TextureId`].
    #[inline]
    pub fn id(&self) -> TextureId {
        self.id
    }

    /// Creates a view of this texture.
    pub fn create_view(&self, desc: &wgpu::TextureViewDescriptor) -> TextureView {
        TextureView::from(self.value.create_view(desc))
    }
}

impl From<wgpu::Texture> for Texture {
    fn from(value: wgpu::Texture) -> Self {
        Texture {
            id: TextureId(Uuid::new_v4()),
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

/// A [`TextureView`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureViewId(Uuid);

/// This type combines wgpu's [`TextureView`](wgpu::TextureView) and
/// [`SwapChainFrame`](wgpu::SwapChainFrame) into the same interface.
#[derive(Clone, Debug)]
pub enum TextureViewValue {
    /// The value is an actual wgpu [`TextureView`](wgpu::TextureView).
    TextureView(Arc<wgpu::TextureView>),
    /// The value is a wgpu [`SwapChainFrame`](wgpu::SwapChainFrame), but dereferences to
    /// a [`TextureView`](wgpu::TextureView).
    SwapChainFrame(Arc<wgpu::SwapChainFrame>),
}

/// Describes a [`Texture`] with its associated metadata required by a pipeline or [`BindGroup`](super::BindGroup).
///
/// May be converted from a [`TextureView`](wgpu::TextureView) or [`SwapChainFrame`](wgpu::SwapChainFrame)
/// or dereferences to a wgpu [`TextureView`](wgpu::TextureView).
#[derive(Clone, Debug)]
pub struct TextureView {
    id: TextureViewId,
    value: TextureViewValue,
}

impl TextureView {
    /// Returns the [`TextureViewId`].
    #[inline]
    pub fn id(&self) -> TextureViewId {
        self.id
    }
}

impl From<wgpu::TextureView> for TextureView {
    fn from(value: wgpu::TextureView) -> Self {
        TextureView {
            id: TextureViewId(Uuid::new_v4()),
            value: TextureViewValue::TextureView(Arc::new(value)),
        }
    }
}

impl From<wgpu::SwapChainFrame> for TextureView {
    fn from(value: wgpu::SwapChainFrame) -> Self {
        TextureView {
            id: TextureViewId(Uuid::new_v4()),
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

/// A [`Sampler`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(Uuid);

/// A Sampler defines how a pipeline will sample from a [`TextureView`].
/// They define image filters (including anisotropy) and address (wrapping) modes, among other things.
///
/// May be converted from and dereferences to a wgpu [`Sampler`](wgpu::Sampler).
/// Can be created via [`RenderDevice::create_sampler`](crate::renderer::render_device::RenderDevice::create_sampler).
#[derive(Clone, Debug)]
pub struct Sampler {
    id: SamplerId,
    value: Arc<wgpu::Sampler>,
}

impl Sampler {
    /// Returns the [`SamplerId`].
    #[inline]
    pub fn id(&self) -> SamplerId {
        self.id
    }
}

impl From<wgpu::Sampler> for Sampler {
    fn from(value: wgpu::Sampler) -> Self {
        Sampler {
            id: SamplerId(Uuid::new_v4()),
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
            id: TextureViewId(Uuid::new_v4()),
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
