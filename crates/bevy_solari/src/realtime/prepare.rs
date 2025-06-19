use super::SolariLighting;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{Buffer, BufferDescriptor, BufferUsages},
    renderer::RenderDevice,
};

/// Size of a Reservoir shader struct in bytes.
const RESERVOIR_STRUCT_SIZE: u64 = 32;

/// Internal rendering resources used for Solari lighting.
#[derive(Component)]
pub struct SolariLightingResources {
    pub reservoirs_a: Buffer,
    pub reservoirs_b: Buffer,
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

        let size = (view_size.x * view_size.y) as u64 * RESERVOIR_STRUCT_SIZE;

        let reservoirs_a = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_reservoirs_a"),
            size,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let reservoirs_b = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_reservoirs_b"),
            size,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        commands.entity(entity).insert(SolariLightingResources {
            reservoirs_a,
            reservoirs_b,
            view_size,
        });
    }
}
