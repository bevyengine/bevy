use std::num::NonZeroU64;

use bevy_app::{CoreStage, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Entity,
    schedule::IntoSystemDescriptor,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    picking::{self, Picking},
    render_phase::{batch_phase_system, BatchedPhaseItem, RenderPhase},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, BufferBindingType, ShaderStages, UniformBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderStage,
};
use bevy_utils::HashMap;

use crate::core_2d::Transparent2d;

pub mod node;

/// Uses the GPU to provide a buffer which allows lookup of entities at a given coordinate.
#[derive(Default)]
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Return early if no render app, this can happen in headless situations.
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            .init_resource::<BatchVertexOffsets>()
            .init_resource::<BatchVertexOffsetsLayout>()
            .add_system_to_stage(RenderStage::Prepare, picking::prepare_picking_targets)
            .add_system_to_stage(
                RenderStage::PhaseSort,
                batch_item_add_range_start.after(batch_phase_system::<Transparent2d>),
            );

        app.add_plugin(ExtractComponentPlugin::<Picking>::default())
            .add_system_to_stage(CoreStage::PreUpdate, picking::map_buffers)
            .add_system_to_stage(CoreStage::PostUpdate, picking::unmap_buffers);
    }
}

/// This system batches the [`PhaseItem`]s of all [`RenderPhase`]s of this type.
// pub fn batch_phase_system<I: BatchedPhaseItem>(mut render_phases: Query<&mut RenderPhase<I>>) {
//     for mut phase in &mut render_phases {
//         phase.batch();
//     }
// }

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub struct BatchVertexOffsets {
    pub offset: HashMap<Entity, BindGroup>,
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct BatchVertexOffsetsLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for BatchVertexOffsetsLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        NonZeroU64::new(std::mem::size_of::<u32>() as u64).unwrap(),
                    ),
                },
                count: None,
            }],
            label: Some("batch_offsets_layout"),
        });

        BatchVertexOffsetsLayout { layout }
    }
}

// Bind group insertion
fn batch_item_add_range_start(
    // mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    bind_group_layout: Res<BatchVertexOffsetsLayout>,
    mut offsets: ResMut<BatchVertexOffsets>,
    render_phases: Query<&RenderPhase<Transparent2d>>,
) {
    offsets.clear();

    for phase in &render_phases {
        for item in &phase.items {
            let Some(range) = item.batch_range().as_ref() else { continue };

            let mut buffer = UniformBuffer::default();

            buffer.set(range.start);
            buffer.write_buffer(&device, &queue);

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("BatchVertexOffset"),
                layout: &bind_group_layout.layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.binding().unwrap(),
                }],
            });

            offsets.insert(item.entity, bind_group);

            // commands.entity(item.entity).insert(BatchVertexOffset {
            //     offset: range.start,
            // });
        }
    }
}
