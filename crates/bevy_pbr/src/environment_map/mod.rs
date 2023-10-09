use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::{
    blit::{BlitPipeline, BlitPipelineKey},
    core_3d::{
        graph::node::{PREPASS, START_MAIN_PASS},
        CORE_3D,
    },
    prelude::Camera3d,
};
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    query::{Or, With},
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
    world::World,
};
use bevy_math::UVec2;
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    render_resource::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType,
        CachedRenderPipelineId, Extent3d, FilterMode, LoadOp, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, SamplerBindingType, SamplerDescriptor,
        Shader, ShaderStages, SpecializedRenderPipelines, Texture, TextureAspect,
        TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{FallbackImage, GpuImage, Image},
    Render, RenderApp, RenderSet,
};
use bevy_utils::EntityHashMap;

use crate::LightProbe;

pub const ENVIRONMENT_MAP_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(154476556247605696);

// FIXME: Compress better.
const CUBEMAP_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

pub const PREPARE_ENVIRONMENT_MAPS: &str = "prepare_environment_maps";

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
                .init_resource::<EnvironmentMapPipelines>()
                .add_systems(
                    Render,
                    prepare_environment_map_pipelines.in_set(RenderSet::Prepare),
                )
                .add_systems(
                    Render,
                    prepare_environment_maps.in_set(RenderSet::PrepareResources),
                )
                .add_render_graph_node::<PrepareEnvironmentMapsNode>(
                    CORE_3D,
                    PREPARE_ENVIRONMENT_MAPS,
                )
                .add_render_graph_edges(
                    CORE_3D,
                    &[PREPASS, PREPARE_ENVIRONMENT_MAPS, START_MAIN_PASS],
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

#[derive(Resource)]
pub struct RenderEnvironmentMaps {
    pub diffuse: EnvironmentMapArray,
    pub specular: EnvironmentMapArray,
}

pub struct EnvironmentMapArray {
    kind: EnvironmentMapKind,
    size: UVec2,

    image: Option<GpuImage>,

    /// A list of references to the top layer of each cubemap.
    ///
    /// We use these to generate mipmaps.
    cubemaps: Vec<(Texture, TextureFormat)>,

    entity_to_cubemap_array_index: EntityHashMap<Entity, u32>,
}

#[derive(Resource, Default)]
pub struct EnvironmentMapPipelines {
    blit: Option<CachedRenderPipelineId>,
}

pub enum EnvironmentMapKind {
    Diffuse,
    Specular,
}

#[derive(Default)]
pub struct PrepareEnvironmentMapsNode;

impl EnvironmentMapLight {
    /// Whether or not all textures necessary to use the environment map
    /// have been loaded by the asset server.
    pub fn is_loaded(&self, images: &RenderAssets<Image>) -> bool {
        images.get(&self.diffuse_map).is_some() && images.get(&self.specular_map).is_some()
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

pub fn prepare_environment_map_pipelines(
    mut environment_map_pipelines: ResMut<EnvironmentMapPipelines>,
    blit_pipeline: Res<BlitPipeline>,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut specialized_render_pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
) {
    if environment_map_pipelines.blit.is_none() {
        environment_map_pipelines.blit = Some(specialized_render_pipelines.specialize(
            &pipeline_cache,
            &*blit_pipeline,
            BlitPipelineKey {
                texture_format: CUBEMAP_TEXTURE_FORMAT,
                samples: 1,
                blend_state: None,
            },
        ));

        // We need to flush the queue now, because it won't get flushed before
        // `prepare_environment_maps`.
        pipeline_cache.process_queue();
    }
}

pub fn prepare_environment_maps(
    reflection_probes: Query<(Entity, &EnvironmentMapLight)>,
    mut render_environment_maps: ResMut<RenderEnvironmentMaps>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
) {
    render_environment_maps.diffuse.clear();
    render_environment_maps.specular.clear();

    // Gather up the reflection probes.
    for (reflection_probe, environment_map_light) in reflection_probes.iter() {
        render_environment_maps.diffuse.add_image(
            reflection_probe,
            &environment_map_light.diffuse_map,
            &images,
        );
        render_environment_maps.specular.add_image(
            reflection_probe,
            &environment_map_light.specular_map,
            &images,
        );
    }

    println!(
        "have {} diffuse maps, {} specular maps",
        render_environment_maps.diffuse.cubemaps.len(),
        render_environment_maps.specular.cubemaps.len()
    );

    // Create the textures.
    if !render_environment_maps.diffuse.is_empty() {
        render_environment_maps
            .diffuse
            .create_texture(&render_device);
    }
    if !render_environment_maps.specular.is_empty() {
        render_environment_maps
            .specular
            .create_texture(&render_device);
    }
}

impl EnvironmentMapArray {
    fn new(kind: EnvironmentMapKind) -> Self {
        Self {
            kind,
            size: UVec2::ZERO,
            entity_to_cubemap_array_index: EntityHashMap::default(),
            cubemaps: vec![],
            image: None,
        }
    }

    fn add_image(
        &mut self,
        entity: Entity,
        image_handle: &Handle<Image>,
        images: &RenderAssets<Image>,
    ) {
        let Some(image) = images.get(image_handle) else { return };

        self.size = self.size.max(image.size.as_uvec2());
        self.entity_to_cubemap_array_index
            .insert(entity, self.cubemaps.len() as u32);

        self.cubemaps
            .push((image.texture.clone(), image.texture_format));
    }

    fn create_texture(&mut self, render_device: &RenderDevice) {
        let mip_level_count = self.mip_level_count();

        let texture = render_device.create_texture(&TextureDescriptor {
            label: match self.kind {
                EnvironmentMapKind::Diffuse => Some("environment_map_diffuse_texture"),
                EnvironmentMapKind::Specular => Some("environment_map_specular_texture"),
            },
            size: Extent3d {
                width: self.size.x,
                height: self.size.y,
                depth_or_array_layers: self.cubemaps.len() as u32 * 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: CUBEMAP_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: match self.kind {
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
            label: match self.kind {
                EnvironmentMapKind::Diffuse => Some("environment_map_diffuse_sampler"),
                EnvironmentMapKind::Specular => Some("environment_map_specular_sampler"),
            },
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        self.image = Some(GpuImage {
            texture,
            texture_view,
            texture_format: CUBEMAP_TEXTURE_FORMAT,
            sampler,
            size: self.size.as_vec2(),
            mip_level_count,
        });
    }

    fn copy_cubemaps_in(
        &self,
        render_context: &mut RenderContext,
        blit_pipeline: &BlitPipeline,
        pipeline_cache: &PipelineCache,
        blit_pipeline_id: CachedRenderPipelineId,
    ) {
        let gpu_image = self
            .image
            .as_ref()
            .expect("`copy_cubemaps_in()` called with no texture present");

        for (cubemap_index, (top_layer_texture, top_layer_texture_format)) in
            self.cubemaps.iter().enumerate()
        {
            for side in 0..6 {
                let mut src_texture_view = top_layer_texture.create_view(&TextureViewDescriptor {
                    label: Some("environment_map_src_layer"),
                    format: Some(*top_layer_texture_format),
                    dimension: Some(TextureViewDimension::D2),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: side,
                    array_layer_count: Some(1),
                });

                for mip_level in 0..self.mip_level_count() {
                    let dest_texture_view = gpu_image.texture.create_view(&TextureViewDescriptor {
                        label: Some("environment_map_dest_texture_view"),
                        format: Some(TextureFormat::Rgba32Float),
                        dimension: Some(TextureViewDimension::D2),
                        aspect: TextureAspect::All,
                        base_mip_level: mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: cubemap_index as u32 * 6 + side,
                        array_layer_count: Some(1),
                    });

                    let attachment = RenderPassColorAttachment {
                        view: &dest_texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Default::default()),
                            store: true,
                        },
                    };

                    let pass_descriptor = RenderPassDescriptor {
                        label: Some("prepare_environment_map"),
                        color_attachments: &[Some(attachment)],
                        depth_stencil_attachment: None,
                    };

                    let bind_group =
                        render_context
                            .render_device()
                            .create_bind_group(&BindGroupDescriptor {
                                label: Some("prepare_environment_map_descriptor"),
                                layout: &blit_pipeline.texture_bind_group,
                                entries: &[
                                    BindGroupEntry {
                                        binding: 0,
                                        resource: BindingResource::TextureView(&src_texture_view),
                                    },
                                    BindGroupEntry {
                                        binding: 1,
                                        resource: BindingResource::Sampler(&blit_pipeline.sampler),
                                    },
                                ],
                            });

                    // Grab the blit pipeline.
                    let pipeline = pipeline_cache
                        .get_render_pipeline(blit_pipeline_id)
                        .expect("No render pipeline found for environment map creation");

                    {
                        let mut render_pass = render_context
                            .command_encoder()
                            .begin_render_pass(&pass_descriptor);
                        render_pass.set_pipeline(pipeline);
                        render_pass.set_bind_group(0, &bind_group, &[]);
                        render_pass.draw(0..3, 0..1);
                    }

                    src_texture_view = dest_texture_view;
                }
            }
        }
    }

    fn mip_level_count(&self) -> u32 {
        self.size.min_element().ilog2()
    }

    pub fn is_empty(&self) -> bool {
        self.cubemaps.is_empty()
    }

    fn clear(&mut self) {
        self.size = UVec2::ZERO;
        self.cubemaps.clear();
        self.entity_to_cubemap_array_index.clear();
    }
}

impl RenderEnvironmentMaps {
    pub fn get_bindings<'r>(
        &'r self,
        fallback_image: &'r FallbackImage,
        bindings: &[u32; 3],
    ) -> [BindGroupEntry<'r>; 3] {
        let diffuse_map = self
            .diffuse
            .image
            .as_ref()
            .unwrap_or(&fallback_image.cube_array);
        let specular_map = self
            .specular
            .image
            .as_ref()
            .unwrap_or(&fallback_image.cube_array);

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

impl Node for PrepareEnvironmentMapsNode {
    fn run(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let environment_maps = world.resource::<RenderEnvironmentMaps>();
        let environment_map_pipelines = world.resource::<EnvironmentMapPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let blit_pipeline = world.resource::<BlitPipeline>();

        let blit_pipeline_id = environment_map_pipelines
            .blit
            .expect("Blit pipeline wasn't prepared");

        if !environment_maps.diffuse.is_empty() {
            environment_maps.diffuse.copy_cubemaps_in(
                render_context,
                blit_pipeline,
                pipeline_cache,
                blit_pipeline_id,
            )
        }

        if !environment_maps.specular.is_empty() {
            environment_maps.specular.copy_cubemaps_in(
                render_context,
                blit_pipeline,
                pipeline_cache,
                blit_pipeline_id,
            )
        }

        Ok(())
    }
}

impl Default for RenderEnvironmentMaps {
    fn default() -> RenderEnvironmentMaps {
        RenderEnvironmentMaps {
            diffuse: EnvironmentMapArray::new(EnvironmentMapKind::Diffuse),
            specular: EnvironmentMapArray::new(EnvironmentMapKind::Specular),
        }
    }
}
