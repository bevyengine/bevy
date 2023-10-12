use std::iter;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
use bevy_core_pipeline::core_3d::Camera3d;
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    query::{Or, With},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_math::vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType,
        CommandEncoderDescriptor, DynamicUniformBuffer, Extent3d, FilterMode, ImageCopyTexture,
        Origin3d, SamplerBindingType, SamplerDescriptor, Shader, ShaderStages, Texture,
        TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
        TextureUsages, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, GpuImage, Image},
    Render, RenderApp, RenderSet,
};
use bevy_utils::{tracing::warn, HashMap};

use crate::LightProbe;

pub const ENVIRONMENT_MAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(154476556247605696);

// FIXME: Don't hardcode this!
const CUBEMAP_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgb9e5Ufloat;

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

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderEnvironmentMaps>()
                .init_resource::<EnvironmentMapMeta>()
                .add_systems(
                    Render,
                    prepare_environment_maps.in_set(RenderSet::PrepareResources),
                )
                .add_systems(
                    Render,
                    prepare_view_environment_map
                        .after(prepare_environment_maps)
                        .in_set(RenderSet::PrepareResources),
                );
        }
    }
}

/// Environment map based ambient lighting representing light from distant scenery.
///
/// When added to a 3D camera, this component adds indirect light to every point of the scene based
/// on an environment cubemap texture, if not overridden by a [LightProbe]. This is similar to
/// [`crate::AmbientLight`], but higher quality, and is intended for outdoor scenes.
///
/// When added to a [LightProbe], the indirect light will be added to all meshes within the bounds
/// of the light probe, overriding the camera's environment map if any.
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

#[derive(Clone, PartialEq, Hash, Eq, Debug)]
pub struct EnvironmentMapLightId {
    pub diffuse: AssetId<Image>,
    pub specular: AssetId<Image>,
}

#[derive(Resource, Default)]
pub struct RenderEnvironmentMaps {
    images: Option<RenderEnvironmentMapImages>,

    /// A list of references to each cubemap.
    cubemaps: Vec<RenderEnvironmentCubemap>,

    /// Maps from asset ID to index in the cubemap.
    pub(crate) light_id_indices: HashMap<EnvironmentMapLightId, i32>,
}

pub struct RenderEnvironmentMapImages {
    diffuse: GpuImage,
    specular: GpuImage,
}

struct RenderEnvironmentCubemap {
    diffuse_texture: Texture,
    specular_texture: Texture,
}

pub enum EnvironmentMapKind {
    Diffuse,
    Specular,
}

#[derive(Default)]
pub struct PrepareEnvironmentMapsNode;

#[derive(Default, Resource)]
pub struct EnvironmentMapMeta {
    // The indices of the view environment map in the diffuse and specular
    // cubemap arrays, used as a fallback in case no reflection probe applies to
    // the mesh. This will be -1 if not present.
    pub view_environment_map_indices: DynamicUniformBuffer<i32>,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
pub struct ViewEnvironmentMapUniformOffset {
    pub offset: u32,
}

impl EnvironmentMapLight {
    /// Whether or not all textures necessary to use the environment map
    /// have been loaded by the asset server.
    pub fn is_loaded(&self, images: &RenderAssets<Image>) -> bool {
        images.get(&self.diffuse_map).is_some() && images.get(&self.specular_map).is_some()
    }

    pub fn id(&self) -> EnvironmentMapLightId {
        EnvironmentMapLightId {
            diffuse: self.diffuse_map.id(),
            specular: self.specular_map.id(),
        }
    }
}

impl ExtractComponent for EnvironmentMapLight {
    type Query = &'static Self;
    type Filter = Or<(With<Camera3d>, With<LightProbe>)>;
    type Out = Self;

    fn extract_component(item: bevy_ecs::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

pub fn get_bind_group_layout_entries(bindings: [u32; 3]) -> [BindGroupLayoutEntry; 3] {
    [
        BindGroupLayoutEntry {
            binding: bindings[0],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::CubeArray,
                multisampled: false,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: bindings[1],
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::CubeArray,
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

pub fn prepare_environment_maps(
    reflection_probes: Query<&EnvironmentMapLight>,
    mut render_environment_maps: ResMut<RenderEnvironmentMaps>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    images: Res<RenderAssets<Image>>,
) {
    // Skip if we have nothing to do.
    if reflection_probes.iter().all(|reflection_probe| {
        render_environment_maps
            .light_id_indices
            .contains_key(&reflection_probe.id())
    }) {
        return;
    }

    // Gather up the reflection probes.
    render_environment_maps.clear();
    for environment_map_light in reflection_probes.iter() {
        render_environment_maps.add_images(environment_map_light, &images);
    }

    println!(
        "have {} environment maps",
        render_environment_maps.cubemaps.len(),
    );

    // Create the textures.

    let (first_diffuse_texture, first_specular_texture) =
        match render_environment_maps.cubemaps.first() {
            None => return,
            Some(first_cubemap) => (
                &first_cubemap.diffuse_texture,
                &first_cubemap.specular_texture,
            ),
        };

    render_environment_maps.images = Some(RenderEnvironmentMapImages {
        diffuse: render_environment_maps.create_image(
            &render_device,
            EnvironmentMapKind::Diffuse,
            first_diffuse_texture.size(),
            first_diffuse_texture.mip_level_count(),
        ),
        specular: render_environment_maps.create_image(
            &render_device,
            EnvironmentMapKind::Specular,
            first_specular_texture.size(),
            first_specular_texture.mip_level_count(),
        ),
    });

    render_environment_maps.copy_cubemaps_in(
        &render_device,
        &render_queue,
        EnvironmentMapKind::Diffuse,
    );
    render_environment_maps.copy_cubemaps_in(
        &render_device,
        &render_queue,
        EnvironmentMapKind::Specular,
    );
}

impl RenderEnvironmentMaps {
    fn add_images(
        &mut self,
        environment_map_light: &EnvironmentMapLight,
        images: &RenderAssets<Image>,
    ) {
        if !environment_map_light.is_loaded(images) {
            return;
        }

        // If we've already added this environment map, then bail out.
        let id = environment_map_light.id();
        if self.light_id_indices.contains_key(&id) {
            return;
        }

        let (Some(diffuse_image), Some(specular_image)) = (
            images.get(&environment_map_light.diffuse_map),
            images.get(&environment_map_light.specular_map)
        ) else { return };

        if let Some(existing_cubemap) = self.cubemaps.first() {
            if !self.check_cubemap_compatibility(
                &existing_cubemap.diffuse_texture,
                &diffuse_image.texture,
            ) || !self.check_cubemap_compatibility(
                &existing_cubemap.specular_texture,
                &specular_image.texture,
            ) {
                return;
            }
        }

        println!(
            "diffuse image size={:?} specular image size={:?}",
            diffuse_image.size, specular_image.size
        );

        self.light_id_indices.insert(id, self.cubemaps.len() as i32);

        self.cubemaps.push(RenderEnvironmentCubemap {
            diffuse_texture: diffuse_image.texture.clone(),
            specular_texture: specular_image.texture.clone(),
        });
    }

    fn check_cubemap_compatibility(
        &self,
        existing_cubemap: &Texture,
        new_cubemap: &Texture,
    ) -> bool {
        if existing_cubemap.size() == new_cubemap.size()
            && existing_cubemap.mip_level_count() == new_cubemap.mip_level_count()
        {
            return true;
        }

        warn!(
            "Ignoring environment map because its size was incompatible with the previous one:
    Previous width: {}, height: {}, mip levels: {}
    This width: {}, height: {}, mip levels: {}",
            existing_cubemap.width(),
            existing_cubemap.height(),
            existing_cubemap.mip_level_count(),
            new_cubemap.width(),
            new_cubemap.height(),
            new_cubemap.mip_level_count(),
        );
        false
    }

    fn create_image(
        &self,
        render_device: &RenderDevice,
        kind: EnvironmentMapKind,
        extents: Extent3d,
        mip_level_count: u32,
    ) -> GpuImage {
        let texture = render_device.create_texture(&TextureDescriptor {
            label: match kind {
                EnvironmentMapKind::Diffuse => Some("environment_map_diffuse_texture"),
                EnvironmentMapKind::Specular => Some("environment_map_specular_texture"),
            },
            size: Extent3d {
                width: extents.width,
                height: extents.height,
                depth_or_array_layers: self.cubemaps.len() as u32 * 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: CUBEMAP_TEXTURE_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: match kind {
                EnvironmentMapKind::Diffuse => Some("environment_map_diffuse_texture_view"),
                EnvironmentMapKind::Specular => Some("environment_map_specular_texture_view"),
            },
            format: Some(CUBEMAP_TEXTURE_FORMAT),
            dimension: Some(TextureViewDimension::CubeArray),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(mip_level_count),
            base_array_layer: 0,
            array_layer_count: Some(self.cubemaps.len() as u32 * 6),
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: match kind {
                EnvironmentMapKind::Diffuse => Some("environment_map_diffuse_sampler"),
                EnvironmentMapKind::Specular => Some("environment_map_specular_sampler"),
            },
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        GpuImage {
            texture,
            texture_view,
            texture_format: CUBEMAP_TEXTURE_FORMAT,
            sampler,
            size: vec2(extents.width as f32, extents.height as f32),
            mip_level_count,
        }
    }

    fn copy_cubemaps_in(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        kind: EnvironmentMapKind,
    ) {
        let environment_map_images = self
            .images
            .as_ref()
            .expect("`copy_cubemaps_in()` called with no texture present");

        let dest_image = match kind {
            EnvironmentMapKind::Diffuse => &environment_map_images.diffuse,
            EnvironmentMapKind::Specular => &environment_map_images.specular,
        };

        let (width, height) = (dest_image.size.x as u32, dest_image.size.y as u32);

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: match kind {
                EnvironmentMapKind::Diffuse => Some("copy_environment_maps_diffuse"),
                EnvironmentMapKind::Specular => Some("copy_environment_maps_specular"),
            },
        });

        for (cubemap_index, cubemap) in self.cubemaps.iter().enumerate() {
            let src_texture = match kind {
                EnvironmentMapKind::Diffuse => &cubemap.diffuse_texture,
                EnvironmentMapKind::Specular => &cubemap.specular_texture,
            };

            println!(
                "src mip count={} dest mip count={}",
                src_texture.mip_level_count(),
                dest_image.mip_level_count
            );

            for mip_level in 0..src_texture.mip_level_count() {
                command_encoder.copy_texture_to_texture(
                    ImageCopyTexture {
                        texture: src_texture,
                        mip_level,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    ImageCopyTexture {
                        texture: &dest_image.texture,
                        mip_level,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: cubemap_index as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    Extent3d {
                        width: width >> mip_level,
                        height: height >> mip_level,
                        depth_or_array_layers: 6,
                    },
                );
            }
        }

        let command_buffer = command_encoder.finish();
        render_queue.submit(iter::once(command_buffer));
    }

    pub fn is_empty(&self) -> bool {
        self.cubemaps.is_empty()
    }

    fn clear(&mut self) {
        self.cubemaps.clear();
        self.light_id_indices.clear();
    }

    pub(crate) fn get_index(&self, environment_map_light_id: &EnvironmentMapLightId) -> i32 {
        match self.light_id_indices.get(environment_map_light_id) {
            Some(&index) => index,
            None => -1,
        }
    }
}

impl RenderEnvironmentMaps {
    pub fn get_bindings<'r>(
        &'r self,
        fallback_image: &'r FallbackImage,
        bindings: &[u32; 3],
    ) -> [BindGroupEntry<'r>; 3] {
        let (diffuse_map, specular_map) = match self.images {
            None => (&fallback_image.cube_array, &fallback_image.cube_array),
            Some(ref images) => (&images.diffuse, &images.specular),
        };

        [
            BindGroupEntry {
                binding: bindings[0],
                resource: BindingResource::TextureView(&diffuse_map.texture_view),
            },
            BindGroupEntry {
                binding: bindings[1],
                resource: BindingResource::TextureView(&specular_map.texture_view),
            },
            BindGroupEntry {
                binding: bindings[2],
                resource: BindingResource::Sampler(&diffuse_map.sampler),
            },
        ]
    }
}

pub fn prepare_view_environment_map(
    mut commands: Commands,
    mut environment_map_meta: ResMut<EnvironmentMapMeta>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    render_environment_maps: ResMut<RenderEnvironmentMaps>,
    views: Query<(Entity, Option<&EnvironmentMapLight>)>,
) {
    let views_iter = views.iter();
    let view_count = views_iter.len();

    let Some(mut writer) = environment_map_meta
        .view_environment_map_indices
        .get_writer(view_count, &render_device, &render_queue) else { return };

    for (view_entity, environment_map_light) in views_iter {
        let environment_map_index = match environment_map_light {
            None => -1,
            Some(environment_map_light) => {
                render_environment_maps.get_index(&environment_map_light.id())
            }
        };

        commands
            .entity(view_entity)
            .insert(ViewEnvironmentMapUniformOffset {
                offset: writer.write(&environment_map_index),
            });
    }
}
