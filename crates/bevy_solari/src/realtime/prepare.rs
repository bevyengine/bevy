use super::SolariLighting;
use bevy_core_pipeline::{core_3d::CORE_3D_DEPTH_FORMAT, deferred::DEFERRED_PREPASS_FORMAT};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_image::ToExtents;
use bevy_math::UVec2;
use bevy_render::{
    camera::{ExtractedCamera, MainPassResolutionOverride},
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, Texture, TextureDescriptor, TextureDimension,
        TextureUsages, TextureView, TextureViewDescriptor,
    },
    renderer::RenderDevice,
};

/// Size of the `LightSample` shader struct in bytes.
const LIGHT_SAMPLE_STRUCT_SIZE: u64 = 8;

/// Size of the `ResolvedLightSamplePacked` shader struct in bytes.
const RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE: u64 = 24;

/// Size of the DI `Reservoir` shader struct in bytes.
const DI_RESERVOIR_STRUCT_SIZE: u64 = 16;

/// Size of the GI `Reservoir` shader struct in bytes.
const GI_RESERVOIR_STRUCT_SIZE: u64 = 48;

pub const LIGHT_TILE_BLOCKS: u64 = 128;
pub const LIGHT_TILE_SAMPLES_PER_BLOCK: u64 = 1024;

/// Internal rendering resources used for Solari lighting.
#[derive(Component)]
pub struct SolariLightingResources {
    pub light_tile_samples: Buffer,
    pub light_tile_resolved_samples: Buffer,
    pub di_reservoirs_a: Buffer,
    pub di_reservoirs_b: Buffer,
    pub gi_reservoirs_a: Buffer,
    pub gi_reservoirs_b: Buffer,
    pub previous_gbuffer: (Texture, TextureView),
    pub previous_depth: (Texture, TextureView),
    pub view_size: UVec2,
}

pub fn prepare_solari_lighting_resources(
    query: Query<
        (
            Entity,
            &ExtractedCamera,
            Option<&SolariLightingResources>,
            Option<&MainPassResolutionOverride>,
        ),
        With<SolariLighting>,
    >,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, camera, solari_lighting_resources, resolution_override) in &query {
        let Some(mut view_size) = camera.physical_viewport_size else {
            continue;
        };
        if let Some(MainPassResolutionOverride(resolution_override)) = resolution_override {
            view_size = *resolution_override;
        }

        if solari_lighting_resources.map(|r| r.view_size) == Some(view_size) {
            continue;
        }

        let light_tile_samples = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_light_tile_samples"),
            size: LIGHT_TILE_BLOCKS * LIGHT_TILE_SAMPLES_PER_BLOCK * LIGHT_SAMPLE_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let light_tile_resolved_samples = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_light_tile_resolved_samples"),
            size: LIGHT_TILE_BLOCKS
                * LIGHT_TILE_SAMPLES_PER_BLOCK
                * RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let di_reservoirs_a = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_di_reservoirs_a"),
            size: (view_size.x * view_size.y) as u64 * DI_RESERVOIR_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let di_reservoirs_b = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_di_reservoirs_b"),
            size: (view_size.x * view_size.y) as u64 * DI_RESERVOIR_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let gi_reservoirs_a = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_gi_reservoirs_a"),
            size: (view_size.x * view_size.y) as u64 * GI_RESERVOIR_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let gi_reservoirs_b = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_gi_reservoirs_b"),
            size: (view_size.x * view_size.y) as u64 * GI_RESERVOIR_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let previous_gbuffer = render_device.create_texture(&TextureDescriptor {
            label: Some("solari_lighting_previous_gbuffer"),
            size: view_size.to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEFERRED_PREPASS_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let previous_gbuffer_view = previous_gbuffer.create_view(&TextureViewDescriptor::default());

        let previous_depth = render_device.create_texture(&TextureDescriptor {
            label: Some("solari_lighting_previous_depth"),
            size: view_size.to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: CORE_3D_DEPTH_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let previous_depth_view = previous_depth.create_view(&TextureViewDescriptor::default());

        commands.entity(entity).insert(SolariLightingResources {
            light_tile_samples,
            light_tile_resolved_samples,
            di_reservoirs_a,
            di_reservoirs_b,
            gi_reservoirs_a,
            gi_reservoirs_b,
            previous_gbuffer: (previous_gbuffer, previous_gbuffer_view),
            previous_depth: (previous_depth, previous_depth_view),
            view_size,
        });
    }
}
