use super::{EnvironmentMapLight, DOWNSAMPLE_SHADER_HANDLE, FILTER_SHADER_HANDLE};
use bevy_asset::Assets;
use bevy_core_pipeline::Skybox;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryItem, With, Without},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::ExtractComponent,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache,
        SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureAspect,
        TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{GpuImage, Image, ImageSampler},
};
use bevy_utils::default;

/// TODO: Docs
#[derive(Component, ExtractComponent, Reflect, Default, Copy, Clone)]
pub struct GenerateEnvironmentMapLight {
    downsampled_cubemap: Option<Handle<Image>>,
}

#[derive(Resource)]
pub struct GenerateEnvironmentMapLightResources {
    downsample_layout: BindGroupLayout,
    filter_layout: BindGroupLayout,
    downsample_pipeline: CachedComputePipelineId,
    filter_pipeline: CachedComputePipelineId,
}

impl FromWorld for GenerateEnvironmentMapLightResources {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let read_texture = BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::Cube,
            multisampled: false,
        };
        let write_texture = BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: TextureFormat::Rg11b10Float,
            view_dimension: TextureViewDimension::D2Array,
        };

        let downsample_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("generate_environment_map_light_downsample_bind_group_layout"),
                entries: &[
                    bgl_entry(0, read_texture),
                    bgl_entry(1, write_texture),
                    bgl_entry(2, BindingType::Sampler(SamplerBindingType::Filtering)),
                ],
            });

        let filter_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("generate_environment_map_light_filter_bind_group_layout"),
            entries: &[
                bgl_entry(0, read_texture),
                bgl_entry(1, write_texture),
                bgl_entry(2, write_texture),
                bgl_entry(3, write_texture),
                bgl_entry(4, write_texture),
                bgl_entry(5, write_texture),
                bgl_entry(6, write_texture),
                bgl_entry(7, write_texture),
                bgl_entry(8, BindingType::Sampler(SamplerBindingType::Filtering)),
            ],
        });

        let downsample_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("generate_environment_map_light_downsample_pipeline".into()),
                layout: vec![downsample_layout.clone()],
                push_constant_ranges: vec![],
                shader: DOWNSAMPLE_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "main".into(),
            });

        let filter_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("generate_environment_map_light_filter_pipeline".into()),
            layout: vec![filter_layout.clone()],
            push_constant_ranges: vec![],
            shader: FILTER_SHADER_HANDLE.typed(),
            shader_defs: vec![],
            entry_point: "main".into(),
        });

        Self {
            downsample_layout,
            filter_layout,
            downsample_pipeline,
            filter_pipeline,
        }
    }
}

pub fn generate_dummy_environment_map_lights_for_skyboxes(
    skyboxes: Query<
        (Entity, &Skybox, &mut GenerateEnvironmentMapLight),
        Without<EnvironmentMapLight>,
    >,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, skybox, gen_env_map_light) in &skyboxes {
        let texture_descriptor = match images.get(&skybox.0) {
            Some(skybox) => skybox.texture_descriptor.clone(),
            None => continue,
        };

        let mut diffuse_map = Image::new_fill(
            texture_descriptor.size,
            texture_descriptor.dimension,
            &[0],
            TextureFormat::Rg11b10Float,
        );
        diffuse_map.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
        diffuse_map.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
            label: Some("generate_environment_map_light_downsample_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..default()
        });

        let mut specular_map = Image::new_fill(
            texture_descriptor.size,
            texture_descriptor.dimension,
            &[0],
            TextureFormat::Rg11b10Float,
        );
        specular_map.texture_descriptor.mip_level_count = 6;
        specular_map.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
        specular_map.sampler_descriptor = ImageSampler::Descriptor(SamplerDescriptor {
            label: Some("generate_environment_map_light_filter_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..default()
        });

        let mut downsampled_cubemap = Image::new_fill(
            Extent3d {
                width: texture_descriptor.size.width / 2,
                height: texture_descriptor.size.height / 2,
                depth_or_array_layers: texture_descriptor.size.depth_or_array_layers,
            },
            texture_descriptor.dimension,
            &[0],
            TextureFormat::Rg11b10Float,
        );
        downsampled_cubemap.texture_descriptor.mip_level_count = 5;
        downsampled_cubemap.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
        gen_env_map_light.downsampled_cubemap = Some(images.add(downsampled_cubemap));

        commands.entity(entity).insert(EnvironmentMapLight {
            diffuse_map: images.add(diffuse_map),
            specular_map: images.add(specular_map),
        });
    }
}

#[derive(Component)]
pub struct GenerateEnvironmentMapLightBindGroups {
    downsample: [BindGroup; 6],
    filter: BindGroup,
    downsampled_cubemap_size: UVec2,
}

pub fn prepare_generate_environment_map_lights_for_skyboxes_bind_groups(
    environment_map_lights: Query<(
        Entity,
        &Skybox,
        &EnvironmentMapLight,
        &GenerateEnvironmentMapLight,
    )>,
    resources: Res<GenerateEnvironmentMapLightResources>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
    images: Res<RenderAssets<Image>>,
) {
    for (entity, skybox, environment_map_light, gen_env_map) in &environment_map_lights {
        let (Some(skybox), Some(diffuse_map), Some(specular_map), Some(downsampled_cubemap)) = (
            images.get(&skybox.0),
            images.get(&environment_map_light.diffuse_map),
            images.get(&environment_map_light.specular_map),
            gen_env_map.downsampled_cubemap.and_then(|t| images.get(t)),
        ) else {
            continue;
        };

        let downsample1 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample1_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&skybox.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(0, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });
        let downsample2 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample2_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view(0, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(1, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });
        let downsample3 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample3_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view(1, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(2, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });
        let downsample4 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample4_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view(2, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(3, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });
        let downsample5 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample5_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view(3, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(4, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });
        let downsample6 = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_downsample6_bind_group"),
            layout: &resources.downsample_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_view(4, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(5, downsampled_cubemap)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&diffuse_map.sampler),
                },
            ],
        });

        let filter = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("generate_environment_map_light_filter_bind_group"),
            layout: &resources.filter_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&downsampled_cubemap.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_view(0, specular_map)),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&texture_view(1, specular_map)),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&texture_view(2, specular_map)),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(&texture_view(3, specular_map)),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(&texture_view(4, specular_map)),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(&texture_view(5, specular_map)),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(&texture_view(6, specular_map)),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::Sampler(&specular_map.sampler),
                },
            ],
        });

        commands
            .entity(entity)
            .insert(GenerateEnvironmentMapLightBindGroups {
                downsample: [
                    downsample1,
                    downsample2,
                    downsample3,
                    downsample4,
                    downsample5,
                    downsample6,
                ],
                filter,
                downsampled_cubemap_size: downsampled_cubemap.size.as_uvec2(),
            });
    }
}

#[derive(Default)]
pub struct GenerateEnvironmentMapLightNode;

impl ViewNode for GenerateEnvironmentMapLightNode {
    type ViewQuery = &'static GenerateEnvironmentMapLightBindGroups;

    fn run(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        bind_groups: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let resources = world.resource::<GenerateEnvironmentMapLightResources>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let (Some(downsample_pipeline), Some(filter_pipeline)) = (
            pipeline_cache.get_compute_pipeline(resources.downsample_pipeline),
            pipeline_cache.get_compute_pipeline(resources.filter_pipeline),
        ) else {
            return Ok(());
        };

        let command_encoder = render_context.command_encoder();
        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("generate_environment_map_light_pass"),
        });

        pass.set_pipeline(downsample_pipeline);
        let mut texture_size = bind_groups.downsampled_cubemap_size;
        for bind_group in &bind_groups.downsample {
            let workgroup_count = div_ceil(texture_size, 8);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(workgroup_count, workgroup_count, 6);
            texture_size /= 2;
        }

        pass.set_pipeline(filter_pipeline);
        pass.set_bind_group(0, &bind_groups.filter, &[]);
        pass.dispatch_workgroups(todo!(), 6, 1);

        Ok(())
    }
}

fn bgl_entry(binding: u32, ty: BindingType) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty,
        count: None,
    }
}

fn texture_view(mip_level: u32, specular_map: &GpuImage) -> TextureView {
    specular_map.texture.create_view(&TextureViewDescriptor {
        label: Some("generate_environment_map_light_texture_view"),
        format: Some(TextureFormat::Rg11b10Float),
        dimension: Some(TextureViewDimension::D3),
        aspect: TextureAspect::All,
        base_mip_level: mip_level,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(6),
    })
}

/// Divide `numerator` by `denominator`, rounded up to the nearest multiple of `denominator`.
fn div_ceil(numerator: u32, denominator: u32) -> u32 {
    (numerator + denominator - 1) / denominator
}
