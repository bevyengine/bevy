mod generate_from_skybox;

use bevy_app::{App, Last, Plugin};
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::{
    core_3d::{self, CORE_3D},
    prelude::Camera3d,
};
use bevy_ecs::{prelude::Component, query::With, schedule::IntoSystemConfigs};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{
        BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType, SamplerBindingType,
        Shader, ShaderStages, TextureSampleType, TextureViewDimension,
    },
    texture::{FallbackImageCubemap, Image},
    Render, RenderApp, RenderSet,
};
use generate_from_skybox::{
    generate_dummy_environment_map_lights_for_skyboxes,
    prepare_generate_environment_map_lights_for_skyboxes_bind_groups,
    GenerateEnvironmentMapLightNode, GenerateEnvironmentMapLightResources,
};
pub use generate_from_skybox::{
    GenerateEnvironmentMapLight, GenerateEnvironmentMapLightTextureFormat,
};

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the generate environment map light render node.
        pub const GENERATE_ENVIRONMENT_MAP_LIGHT: &str = "generate_environment_map_light";
    }
}

pub const ENVIRONMENT_MAP_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 154476556247605696);
pub const DOWNSAMPLE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 254476556247605696);
pub const FILTER_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 354476556247605696);
pub const DIFFUSE_CONVOLUTION_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 454476556247605696);

pub struct EnvironmentMapLightPlugin;

impl Plugin for EnvironmentMapLightPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            DOWNSAMPLE_SHADER_HANDLE,
            "downsample.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, FILTER_SHADER_HANDLE, "filter.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            DIFFUSE_CONVOLUTION_SHADER_HANDLE,
            "diffuse_convolution.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<EnvironmentMapLight>()
            .register_type::<GenerateEnvironmentMapLight>()
            .register_type::<GenerateEnvironmentMapLightTextureFormat>()
            .add_plugins(ExtractComponentPlugin::<EnvironmentMapLight>::default())
            .add_plugins(ExtractComponentPlugin::<GenerateEnvironmentMapLight>::default());
    }

    fn finish(&self, app: &mut App) {
        if app.get_sub_app(RenderApp).is_err() {
            return;
        }

        app.add_systems(Last, generate_dummy_environment_map_lights_for_skyboxes);

        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<GenerateEnvironmentMapLightNode>>(
                CORE_3D,
                draw_3d_graph::node::GENERATE_ENVIRONMENT_MAP_LIGHT,
            )
            .add_render_graph_edges(
                CORE_3D,
                &[
                    draw_3d_graph::node::GENERATE_ENVIRONMENT_MAP_LIGHT,
                    core_3d::graph::node::START_MAIN_PASS,
                ],
            )
            .init_resource::<GenerateEnvironmentMapLightResources>()
            .add_systems(
                Render,
                prepare_generate_environment_map_lights_for_skyboxes_bind_groups
                    .in_set(RenderSet::Prepare),
            );
    }
}

/// Environment map based ambient lighting representing light from distant scenery.
///
/// When added to a 3D camera, this component adds indirect light
/// to every point of the scene (including inside, enclosed areas) based on
/// an environment cubemap texture. This is similar to [`crate::AmbientLight`], but
/// higher quality, and is intended for outdoor scenes.
///
/// The environment map must be prefiltered into a diffuse and specular cubemap based on the
/// [split-sum approximation](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf).
///
/// To prefilter your environment map, you can either:
/// * Use the [`GenerateEnvironmentMapLight`] component at runtime.
/// * Use `KhronosGroup`'s [glTF-IBL-Sampler](https://github.com/KhronosGroup/glTF-IBL-Sampler) to prefilter it offline.
/// The diffuse map uses the Lambertian distribution, and the specular map uses the GGX distribution.
#[derive(Component, Reflect, Clone)]
pub struct EnvironmentMapLight {
    pub diffuse_map: Handle<Image>,
    pub specular_map: Handle<Image>,
}

impl EnvironmentMapLight {
    /// Whether or not all textures necessary to use the environment map
    /// have been loaded by the asset server.
    pub fn is_loaded(&self, images: &RenderAssets<Image>) -> bool {
        images.get(&self.diffuse_map).is_some() && images.get(&self.specular_map).is_some()
    }
}

impl ExtractComponent for EnvironmentMapLight {
    type Query = &'static Self;
    type Filter = With<Camera3d>;
    type Out = Self;

    fn extract_component(item: bevy_ecs::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

pub fn get_bindings<'a>(
    environment_map_light: Option<&EnvironmentMapLight>,
    images: &'a RenderAssets<Image>,
    fallback_image_cubemap: &'a FallbackImageCubemap,
    bindings: [u32; 3],
) -> [BindGroupEntry<'a>; 3] {
    let (diffuse_map, specular_map, sampler) = match (
        environment_map_light.and_then(|env_map| images.get(&env_map.diffuse_map)),
        environment_map_light.and_then(|env_map| images.get(&env_map.specular_map)),
    ) {
        (Some(diffuse_map), Some(specular_map)) => (
            &diffuse_map.texture_view,
            &specular_map.texture_view,
            &specular_map.sampler,
        ),
        _ => (
            &fallback_image_cubemap.texture_view,
            &fallback_image_cubemap.texture_view,
            &fallback_image_cubemap.sampler,
        ),
    };

    [
        BindGroupEntry {
            binding: bindings[0],
            resource: BindingResource::TextureView(diffuse_map),
        },
        BindGroupEntry {
            binding: bindings[1],
            resource: BindingResource::TextureView(specular_map),
        },
        BindGroupEntry {
            binding: bindings[2],
            resource: BindingResource::Sampler(sampler),
        },
    ]
}

pub fn get_bind_group_layout_entries(bindings: [u32; 3]) -> [BindGroupLayoutEntry; 3] {
    [
        BindGroupLayoutEntry {
            binding: bindings[0],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::Cube,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: bindings[1],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::Cube,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: bindings[2],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
    ]
}
