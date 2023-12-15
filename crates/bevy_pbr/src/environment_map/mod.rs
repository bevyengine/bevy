//! Environment maps and reflection probes.

use std::{num::NonZeroU32, ops::Deref};

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_ecs::{component::Component, query::QueryItem, system::lifetimeless::Read};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_instances::{ExtractInstance, ExtractInstancesPlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types, BindGroupLayoutEntryBuilder, IntoBindingArray, Sampler, SamplerBindingType,
        Shader, TextureSampleType, TextureView,
    },
    texture::{FallbackImage, Image},
    RenderApp,
};
use bevy_utils::HashMap;

/// A handle to the environment map helper shader.
pub const ENVIRONMENT_MAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(154476556247605696);

const MAX_REFLECTION_PROBES: u32 = 32;

pub struct EnvironmentMapPlugin;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvironmentMapIds {
    pub diffuse: AssetId<Image>,
    pub specular: AssetId<Image>,
}

#[derive(Clone, Component, Reflect)]
pub struct EnvironmentMapLight {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
}

#[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
#[derive(Component, Default)]
pub struct RenderViewEnvironmentMaps {
    binding_index_to_cubemap: Vec<EnvironmentMapIds>,
    cubemap_to_binding_index: HashMap<EnvironmentMapIds, u32>,
}

#[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
#[derive(Component, Default)]
pub struct RenderViewEnvironmentMaps {
    cubemap: Option<EnvironmentMapIds>,
}

#[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
pub(crate) struct RenderViewBindGroupEntries<'a> {
    /// This is a vector of `wgpu::TextureView`s. But we don't want to import
    /// `wgpu` in this crate, so we refer to it indirectly like this.
    diffuse_texture_views: Vec<&'a <TextureView as Deref>::Target>,
    specular_texture_views: Vec<&'a <TextureView as Deref>::Target>,
    pub(crate) sampler: &'a Sampler,
}

#[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
pub(crate) struct RenderViewBindGroupEntries<'a> {
    /// This is a `wgpu::TextureView`. But we don't want to import `wgpu` in
    /// this crate, so we refer to it indirectly like this.
    diffuse_texture_view: &'a TextureView,
    specular_texture_view: &'a TextureView,
    pub(crate) sampler: &'a Sampler,
}

impl Plugin for EnvironmentMapPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_plugins(ExtractInstancesPlugin::<EnvironmentMapIds>::new());
    }
}

impl ExtractInstance for EnvironmentMapIds {
    type Query = Read<EnvironmentMapLight>;

    type Filter = ();

    fn extract(item: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(EnvironmentMapIds {
            diffuse: item.diffuse_map.id(),
            specular: item.specular_map.id(),
        })
    }
}

impl RenderViewEnvironmentMaps {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
impl RenderViewEnvironmentMaps {
    pub fn is_empty(&self) -> bool {
        self.binding_index_to_cubemap.is_empty()
    }

    pub fn get_or_insert_cubemap(&mut self, cubemap_id: &EnvironmentMapIds) -> u32 {
        *self
            .cubemap_to_binding_index
            .entry(*cubemap_id)
            .or_insert_with(|| {
                let index = self.binding_index_to_cubemap.len() as u32;
                self.binding_index_to_cubemap.push(*cubemap_id);
                index
            })
    }
}

#[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
impl RenderViewEnvironmentMaps {
    pub fn is_empty(&self) -> bool {
        self.cubemap.is_none()
    }

    pub fn get_or_insert_cubemap(&mut self, cubemap_id: &EnvironmentMapIds) -> u32 {
        self.cubemap = Some(*cubemap_id);
        0
    }
}

#[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
pub(crate) fn get_bind_group_layout_entries() -> [BindGroupLayoutEntryBuilder; 3] {
    [
        binding_types::texture_cube(TextureSampleType::Float { filterable: true })
            .count(NonZeroU32::new(MAX_REFLECTION_PROBES).unwrap()),
        binding_types::texture_cube(TextureSampleType::Float { filterable: true })
            .count(NonZeroU32::new(MAX_REFLECTION_PROBES).unwrap()),
        binding_types::sampler(SamplerBindingType::Filtering),
    ]
}

#[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
pub(crate) fn get_bind_group_layout_entries() -> [BindGroupLayoutEntryBuilder; 3] {
    [
        binding_types::texture_cube(TextureSampleType::Float { filterable: true }),
        binding_types::texture_cube(TextureSampleType::Float { filterable: true }),
        binding_types::sampler(SamplerBindingType::Filtering),
    ]
}

impl<'a> RenderViewBindGroupEntries<'a> {
    #[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
    pub(crate) fn get(
        render_view_environment_maps: Option<&RenderViewEnvironmentMaps>,
        images: &'a RenderAssets<Image>,
        fallback_image: &'a FallbackImage,
    ) -> RenderViewBindGroupEntries<'a> {
        let mut diffuse_texture_views = vec![];
        let mut specular_texture_views = vec![];
        let mut sampler = None;

        if let Some(environment_maps) = render_view_environment_maps {
            for &cubemap_id in &environment_maps.binding_index_to_cubemap {
                add_texture_view(
                    &mut diffuse_texture_views,
                    &mut sampler,
                    cubemap_id.diffuse,
                    images,
                    fallback_image,
                );
                add_texture_view(
                    &mut specular_texture_views,
                    &mut sampler,
                    cubemap_id.specular,
                    images,
                    fallback_image,
                );
            }
        }

        // Need at least one texture.
        if diffuse_texture_views.is_empty() {
            diffuse_texture_views.push(&*fallback_image.cube.texture_view);
            specular_texture_views.push(&*fallback_image.cube.texture_view);
        }

        RenderViewBindGroupEntries {
            diffuse_texture_views,
            specular_texture_views,
            sampler: sampler.unwrap_or(&fallback_image.cube.sampler),
        }
    }

    #[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
    pub(crate) fn get(
        render_view_environment_maps: Option<&RenderViewEnvironmentMaps>,
        images: &'a RenderAssets<Image>,
        fallback_image: &'a FallbackImage,
    ) -> RenderViewBindGroupEntries<'a> {
        if let Some(&RenderViewEnvironmentMaps {
            cubemap: Some(ref cubemap),
        }) = render_view_environment_maps
        {
            if let (Some(diffuse_image), Some(specular_image)) =
                (images.get(cubemap.diffuse), images.get(cubemap.specular))
            {
                return RenderViewBindGroupEntries {
                    diffuse_texture_view: &diffuse_image.texture_view,
                    specular_texture_view: &specular_image.texture_view,
                    sampler: &diffuse_image.sampler,
                };
            }
        }

        RenderViewBindGroupEntries {
            diffuse_texture_view: &fallback_image.cube.texture_view,
            specular_texture_view: &fallback_image.cube.texture_view,
            sampler: &fallback_image.cube.sampler,
        }
    }
}

fn add_texture_view<'a>(
    texture_views: &mut Vec<&'a <TextureView as Deref>::Target>,
    sampler: &mut Option<&'a Sampler>,
    image_id: AssetId<Image>,
    images: &'a RenderAssets<Image>,
    fallback_image: &'a FallbackImage,
) {
    match images.get(image_id) {
        None => texture_views.push(&*fallback_image.cube.texture_view),
        Some(image) => {
            if sampler.is_none() {
                *sampler = Some(&image.sampler);
            }
            texture_views.push(&*image.texture_view);
        }
    }
}

#[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
impl<'a> RenderViewBindGroupEntries<'a> {
    pub(crate) fn diffuse_texture_views(&'a self) -> &'a [&'a <TextureView as Deref>::Target] {
        self.diffuse_texture_views.as_slice()
    }

    pub(crate) fn specular_texture_views(&'a self) -> &'a [&'a <TextureView as Deref>::Target] {
        self.specular_texture_views.as_slice()
    }
}

#[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
impl<'a> RenderViewBindGroupEntries<'a> {
    pub(crate) fn diffuse_texture_views(&self) -> &'a TextureView {
        &self.diffuse_texture_view
    }

    pub(crate) fn specular_texture_views(&self) -> &'a TextureView {
        &self.specular_texture_view
    }
}
