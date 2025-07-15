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
    camera::ExtractedCamera,
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, Texture, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    },
    renderer::RenderDevice,
};

/// Size of a GI Reservoir shader struct in bytes.
const GI_RESERVOIR_STRUCT_SIZE: u64 = 48;

/// Internal rendering resources used for Solari lighting.
#[derive(Component)]
pub struct SolariLightingResources {
    pub di_reservoirs_a: (Texture, TextureView, Texture, TextureView),
    pub di_reservoirs_b: (Texture, TextureView, Texture, TextureView),
    pub gi_reservoirs_a: Buffer,
    pub gi_reservoirs_b: Buffer,
    pub previous_gbuffer: (Texture, TextureView),
    pub previous_depth: (Texture, TextureView),
    pub view_size: UVec2,
}

pub fn prepare_solari_lighting_resources(
    query: Query<
        (Entity, &ExtractedCamera, Option<&SolariLightingResources>),
        With<SolariLighting>,
    >,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, camera, solari_lighting_resources) in &query {
        let Some(view_size) = camera.physical_viewport_size else {
            continue;
        };

        if solari_lighting_resources.map(|r| r.view_size) == Some(view_size) {
            continue;
        }

        let di_reservoir = |label| {
            let tex = render_device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: view_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&TextureViewDescriptor::default());
            (tex, view)
        };

        let di_reservoirs_a_1 = di_reservoir("solari_lighting_di_reservoirs_a_1");
        let di_reservoirs_a_2 = di_reservoir("solari_lighting_di_reservoirs_a_2");
        let di_reservoirs_b_1 = di_reservoir("solari_lighting_di_reservoirs_b_1");
        let di_reservoirs_b_2 = di_reservoir("solari_lighting_di_reservoirs_b_2");

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
            di_reservoirs_a: (
                di_reservoirs_a_1.0,
                di_reservoirs_a_1.1,
                di_reservoirs_a_2.0,
                di_reservoirs_a_2.1,
            ),
            di_reservoirs_b: (
                di_reservoirs_b_1.0,
                di_reservoirs_b_1.1,
                di_reservoirs_b_2.0,
                di_reservoirs_b_2.1,
            ),
            gi_reservoirs_a,
            gi_reservoirs_b,
            previous_gbuffer: (previous_gbuffer, previous_gbuffer_view),
            previous_depth: (previous_depth, previous_depth_view),
            view_size,
        });
    }
}
