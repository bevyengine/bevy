use super::{EnvironmentMapLight, DOWNSAMPLE_SHADER_HANDLE, FILTER_SHADER_HANDLE};
use bevy_asset::Assets;
use bevy_core_pipeline::Skybox;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::Without,
    system::{Commands, Query, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::ExtractComponent,
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        CachedComputePipelineId, ComputePipelineDescriptor, PipelineCache, SamplerBindingType,
        ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::Image,
};

/// TODO: Docs
#[derive(Component, ExtractComponent, Reflect, Copy, Clone)]
pub struct GenerateEnvironmentMapLight;

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
            format: TextureFormat::Rgba16Float,
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
                bgl_entry(7, BindingType::Sampler(SamplerBindingType::Filtering)),
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
    skyboxes: Query<(Entity, &Skybox), Without<EnvironmentMapLight>>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, skybox) in &skyboxes {
        let texture_descriptor = match images.get(&skybox.0) {
            Some(skybox) => skybox.texture_descriptor.clone(),
            None => continue,
        };

        let diffuse_map = Image::new_fill(
            texture_descriptor.size,
            texture_descriptor.dimension,
            &[0],
            TextureFormat::Rgba16Float,
        );

        let mut specular_map = Image::new_fill(
            texture_descriptor.size,
            texture_descriptor.dimension,
            &[0],
            TextureFormat::Rgba16Float,
        );
        specular_map.texture_descriptor.mip_level_count = 7;

        commands.entity(entity).insert(EnvironmentMapLight {
            diffuse_map: images.add(diffuse_map),
            specular_map: images.add(specular_map),
        });
    }
}

#[derive(Component)]
struct GenerateEnvironmentMapLightBindGroup(BindGroup);

pub fn prepare_generate_environment_map_lights_for_skyboxes_bind_groups() {
    todo!()
}

// TODO: Node

fn bgl_entry(binding: u32, ty: BindingType) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
        binding,
        visibility: ShaderStages::COMPUTE,
        ty,
        count: None,
    }
}
