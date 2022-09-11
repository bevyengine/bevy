use bevy_utils::Uuid;
use std::{ops::Deref, sync::Arc};

use crate::render_resource::resource_macros::*;

/// A [`Texture`] identifier.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct TextureId(Uuid);

/// A GPU-accessible texture.
///
/// May be converted from and dereferences to a wgpu [`Texture`](wgpu::Texture).
/// Can be created via [`RenderDevice::create_texture`](crate::renderer::RenderDevice::create_texture).
#[derive(Clone, Debug)]
pub struct Texture {
    id: TextureId,
    value: render_resource_type!(wgpu::Texture),
}

impl Texture {
    /// Returns the [`TextureId`].
    #[inline]
    pub fn id(&self) -> TextureId {
        self.id
    }

    /// Creates a view of this texture.
    pub fn create_view(&self, desc: &wgpu::TextureViewDescriptor) -> TextureView {
        TextureView::from(self.value().create_view(desc))
    }

    fn value(&self) -> &wgpu::Texture {
        render_resource_ref!(&self.value, wgpu::Texture)
    }
}

impl From<wgpu::Texture> for Texture {
    fn from(value: wgpu::Texture) -> Self {
        Texture {
            id: TextureId(Uuid::new_v4()),
            value: render_resource_new!(value),
        }
    }
}

impl Deref for Texture {
    type Target = wgpu::Texture;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        render_resource_drop!(&mut self.value, wgpu::Texture);
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
    TextureView(render_resource_type!(wgpu::TextureView)),

    /// The value is a wgpu [`SurfaceTexture`](wgpu::SurfaceTexture), but dereferences to
    /// a [`TextureView`](wgpu::TextureView).
    SurfaceTexture {
        // NOTE: The order of these fields is important because the view must be dropped before the
        // frame is dropped
        view: render_resource_type!(wgpu::TextureView),
        texture: Option<render_resource_type!(wgpu::SurfaceTexture)>,
    },
}

/// Describes a [`Texture`] with its associated metadata required by a pipeline or [`BindGroup`](super::BindGroup).
///
/// May be converted from a [`TextureView`](wgpu::TextureView) or [`SurfaceTexture`](wgpu::SurfaceTexture)
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

    /// Returns the [`SurfaceTexture`](wgpu::SurfaceTexture) of the texture view if it is of that type.
    #[inline]
    pub fn take_surface_texture(mut self) -> Option<wgpu::SurfaceTexture> {
        if let TextureViewValue::SurfaceTexture { texture, .. } = &mut self.value {
            let texture = texture.take();
            if let Some(texture) = texture {
                return render_resource_try_unwrap!(texture, wgpu::SurfaceTexture);
            }
        }

        None
    }
}

impl From<wgpu::TextureView> for TextureView {
    fn from(value: wgpu::TextureView) -> Self {
        TextureView {
            id: TextureViewId(Uuid::new_v4()),
            value: TextureViewValue::TextureView(render_resource_new!(value)),
        }
    }
}

impl From<wgpu::SurfaceTexture> for TextureView {
    fn from(value: wgpu::SurfaceTexture) -> Self {
        let view = render_resource_new!(value.texture.create_view(&Default::default()));
        let texture = Some(render_resource_new!(value));

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
            TextureViewValue::TextureView(value) => render_resource_ref!(value, wgpu::TextureView),
            TextureViewValue::SurfaceTexture { view, .. } => {
                render_resource_ref!(view, wgpu::TextureView)
            }
        }
    }
}

impl Drop for TextureView {
    fn drop(&mut self) {
        match &mut self.value {
            TextureViewValue::TextureView(value) => {
                render_resource_drop!(value, wgpu::TextureView);
            }
            TextureViewValue::SurfaceTexture { texture, view } => {
                if let Some(texture) = texture {
                    render_resource_drop!(texture, wgpu::SurfaceTexture);
                }
                render_resource_drop!(view, wgpu::TextureView);
            }
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
/// Can be created via [`RenderDevice::create_sampler`](crate::renderer::RenderDevice::create_sampler).
#[derive(Clone, Debug)]
pub struct Sampler {
    id: SamplerId,
    value: render_resource_type!(wgpu::Sampler),
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
            value: render_resource_new!(value),
        }
    }
}

impl Deref for Sampler {
    type Target = wgpu::Sampler;

    #[inline]
    fn deref(&self) -> &Self::Target {
        render_resource_ref!(&self.value, wgpu::Sampler)
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        render_resource_drop!(&mut self.value, wgpu::Sampler);
    }
}
