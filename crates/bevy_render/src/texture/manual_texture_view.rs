use bevy_camera::ManualTextureViewHandle;
use bevy_ecs::{prelude::Component, resource::Resource};
use bevy_image::BevyDefault;
use bevy_math::UVec2;
use bevy_platform::collections::HashMap;
use bevy_render_macros::ExtractResource;
use wgpu::TextureFormat;

use crate::render_resource::TextureView;

/// A manually managed [`TextureView`] for use as a [`bevy_camera::RenderTarget`].
#[derive(Debug, Clone, Component)]
pub struct ManualTextureView {
    pub texture_view: TextureView,
    pub size: UVec2,
    pub format: TextureFormat,
}

impl ManualTextureView {
    pub fn with_default_format(texture_view: TextureView, size: UVec2) -> Self {
        Self {
            texture_view,
            size,
            format: TextureFormat::bevy_default(),
        }
    }
}

/// Stores manually managed [`ManualTextureView`]s for use as a [`bevy_camera::RenderTarget`].
#[derive(Default, Clone, Resource, ExtractResource)]
pub struct ManualTextureViews(HashMap<ManualTextureViewHandle, ManualTextureView>);

impl core::ops::Deref for ManualTextureViews {
    type Target = HashMap<ManualTextureViewHandle, ManualTextureView>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for ManualTextureViews {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
