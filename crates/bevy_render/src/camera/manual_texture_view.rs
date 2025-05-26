use crate::{extract_resource::ExtractResource, render_resource::TextureView};
use bevy_ecs::{prelude::Component, reflect::ReflectComponent, resource::Resource};
use bevy_image::BevyDefault as _;
use bevy_math::UVec2;
use bevy_platform::collections::HashMap;
use bevy_reflect::prelude::*;
use wgpu::TextureFormat;

/// A unique id that corresponds to a specific [`ManualTextureView`] in the [`ManualTextureViews`] collection.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Component, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Hash, Clone)]
pub struct ManualTextureViewHandle(pub u32);

/// A manually managed [`TextureView`] for use as a [`crate::camera::RenderTarget`].
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

/// Stores manually managed [`ManualTextureView`]s for use as a [`crate::camera::RenderTarget`].
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
