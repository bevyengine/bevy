//! Environment maps and reflection probes.

use std::num::NonZeroU32;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_ecs::{component::Component, query::QueryItem, system::lifetimeless::Read};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_instances::{ExtractInstance, ExtractInstancesPlugin},
    render_resource::{
        binding_types, BindGroupLayoutEntryBuilder, SamplerBindingType, Shader, TextureSampleType,
    },
    texture::Image,
    RenderApp,
};
use bevy_utils::HashMap;

/// A handle to the environment map helper shader.
pub const ENVIRONMENT_MAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(154476556247605696);

const MAX_REFLECTION_PROBES: u32 = 32;

pub struct EnvironmentMapPlugin;

#[derive(Clone, Copy)]
pub struct EnvironmentMapIds {
    pub diffuse: AssetId<Image>,
    pub specular: AssetId<Image>,
}

#[derive(Clone, Component, Reflect)]
pub struct EnvironmentMapLight {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
}

#[derive(Component, Default)]
pub struct RenderViewEnvironmentMaps {
    pub binding_index_to_cubemap: Vec<AssetId<Image>>,
    pub cubemap_to_binding_index: HashMap<AssetId<Image>, u32>,
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

    pub fn is_empty(&self) -> bool {
        self.binding_index_to_cubemap.is_empty()
    }

    pub fn get_or_insert_cubemap(&mut self, cubemap_id: &AssetId<Image>) -> u32 {
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

pub fn get_bind_group_layout_entries() -> [BindGroupLayoutEntryBuilder; 2] {
    [
        binding_types::texture_cube(TextureSampleType::Float { filterable: true })
            .count(NonZeroU32::new(MAX_REFLECTION_PROBES).unwrap()),
        binding_types::sampler(SamplerBindingType::Filtering),
    ]
}
