use bevy_utils::Uuid;
use std::{ops::Deref, sync::Arc};

/// A [`Texture`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Uuid);

/// A GPU-accessible texture.
///
/// Although most-commonly used to store data about surface properties, textures can also be used
/// as general purpose storage for 1-, 2-, or 3-dimensional data.
///
/// It is created by setting up a [`TextureDescriptor`](crate::render_resource::TextureDescriptor)
/// and then calling [`RenderDevice::create_texture`](crate::renderer::RenderDevice::create_texture) or  
/// [`RenderDevice::create_texture_with_data`](crate::renderer::RenderDevice::create_texture_with_data).
///
/// Note that a closely related data structure is [`Image`](crate::texture::Image). It can be thought of a CPU-side analogue of
/// [`Texture`](crate::render_resource::Texture), containing the data that will eventually be uploaded to a GPU-side
/// [`Texture`](crate::render_resource::Texture).
///
/// For general information about textures, see the documentation for [`wgpu::Texture`](wgpu::Texture) and the
/// WebGPU specification entry on [`GPUTexture`]. A [`Texture`] may be converted from and dereferences into a
/// [`wgpu::Texture`](wgpu::Texture).
///
/// [`GPUTexture`]: https://gpuweb.github.io/gpuweb/#texture-interface
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

    /// Creates a [`TextureView`](crate::render_resource::TextureView) of this texture.
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
/// [`SurfaceTexture`](wgpu::SurfaceTexture) into the same interface.
#[derive(Clone, Debug)]
pub enum TextureViewValue {
    /// The value is an actual wgpu [`TextureView`](wgpu::TextureView).
    TextureView(Arc<wgpu::TextureView>),

    /// The value is a wgpu [`SurfaceTexture`](wgpu::SurfaceTexture), but dereferences to
    /// a [`TextureView`](wgpu::TextureView).
    SurfaceTexture {
        // NOTE: The order of these fields is important because the view must be dropped before the
        // frame is dropped
        view: Arc<wgpu::TextureView>,
        texture: Arc<wgpu::SurfaceTexture>,
    },
}

/// Describes a [`Texture`] with the associated metadata required by a pipeline or [`BindGroup`](super::BindGroup).
///
/// It is created by setting up a [`TextureViewDescriptor`](crate::render_resource::TextureViewDescriptor), and
/// calling the [`create_view`](crate::render_resource::Texture::create_view) method on a
/// [`Texture`](crate::render_resource::Texture).
///
/// Can be converted from [`wgpu::TextureView`](wgpu::TextureView) or wgpu [`wgpu::SurfaceTexture`](wgpu::SurfaceTexture). It can
/// be dereferenced into a [`wgpu::TextureView`](wgpu::TextureView).
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

    /// Returns the [`SurfaceTexture`](wgpu::SurfaceTexture) of the texture view if it is of that type.
    #[inline]
    pub fn take_surface_texture(self) -> Option<wgpu::SurfaceTexture> {
        match self.value {
            TextureViewValue::TextureView(_) => None,
            TextureViewValue::SurfaceTexture { texture, .. } => Arc::try_unwrap(texture).ok(),
        }
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

impl From<wgpu::SurfaceTexture> for TextureView {
    fn from(value: wgpu::SurfaceTexture) -> Self {
        let texture = Arc::new(value);
        let view = Arc::new(texture.texture.create_view(&Default::default()));

        TextureView {
            id: TextureViewId(Uuid::new_v4()),
            value: TextureViewValue::SurfaceTexture { texture, view },
        }
    }
}

impl Deref for TextureView {
    type Target = wgpu::TextureView;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match &self.value {
            TextureViewValue::TextureView(value) => value,
            TextureViewValue::SurfaceTexture { view, .. } => view,
        }
    }
}

/// A [`Sampler`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct SamplerId(Uuid);

/// A Sampler defines how a pipeline will sample from a [`TextureView`].
/// They define image filters (including anisotropy) and address (wrapping) modes, among other things.
///
/// It is created by setting up a [`SamplerDescriptor`](crate::render_resource::SamplerDescriptor), and a call to
/// [`RenderDevice::create_sampler`](crate::renderer::RenderDevice::create_sampler).
///
/// May be converted from and dereferences into a [`wgpu::Sampler`](wgpu::Sampler).
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
