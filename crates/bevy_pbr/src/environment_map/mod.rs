use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle, HandleUntyped};
use bevy_core_pipeline::prelude::Camera3d;
use bevy_ecs::{prelude::Component, query::With};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType, SamplerBindingType,
        Shader, ShaderStages, TextureSampleType, TextureViewDimension,
    },
    texture::{FallbackImageCubemap, Image},
};

pub const ENVIRONMENT_MAP_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 154476556247605696);

pub struct EnvironmentMapPlugin;

impl Plugin for EnvironmentMapPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<EnvironmentMapLight>()
            .add_plugins(ExtractComponentPlugin::<EnvironmentMapLight>::default());
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
/// To prefilter your environment map, you can use `KhronosGroup`'s [glTF-IBL-Sampler](https://github.com/KhronosGroup/glTF-IBL-Sampler).
/// The diffuse map uses the Lambertian distribution, and the specular map uses the GGX distribution.
///
/// `KhronosGroup` also has several prefiltered environment maps that can be found [here](https://github.com/KhronosGroup/glTF-Sample-Environments).
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
    let (diffuse_map, specular_map) = match (
        environment_map_light.and_then(|env_map| images.get(&env_map.diffuse_map)),
        environment_map_light.and_then(|env_map| images.get(&env_map.specular_map)),
    ) {
        (Some(diffuse_map), Some(specular_map)) => {
            (&diffuse_map.texture_view, &specular_map.texture_view)
        }
        _ => (
            &fallback_image_cubemap.texture_view,
            &fallback_image_cubemap.texture_view,
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
            resource: BindingResource::Sampler(&fallback_image_cubemap.sampler),
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
