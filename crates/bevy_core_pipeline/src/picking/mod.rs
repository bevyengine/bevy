use bevy_app::{CoreStage, Plugin};
use bevy_asset::HandleUntyped;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Entity,
    system::Resource,
    world::{FromWorld, World},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::ExtractComponentPlugin,
    picking::{self, Picking},
    render_resource::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, Shader, ShaderStages, ShaderType,
    },
    renderer::RenderDevice,
    RenderApp, RenderStage,
};
use bytemuck::{Pod, Zeroable};

pub mod node;

pub const PICKING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7934005773504148195);

/// Uses the GPU to provide a buffer which allows lookup of entities at a given coordinate.
#[derive(Default)]
pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Return early if no render app, this can happen in headless situations.
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app
            // .init_resource::<BatchVertexOffsets>()
            // .init_resource::<BatchVertexOffsetsLayout>()
            .init_resource::<EntityIndexLayout>()
            .add_system_to_stage(RenderStage::Prepare, picking::prepare_picking_targets);
        // .add_system_to_stage(
        //     RenderStage::PhaseSort,
        //     batch_item_add_range_start.after(batch_phase_system::<Transparent2d>),
        // );

        app.add_plugin(ExtractComponentPlugin::<Picking>::default())
            .add_system_to_stage(CoreStage::PreUpdate, picking::map_buffers)
            .add_system_to_stage(CoreStage::PostUpdate, picking::unmap_buffers);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, ShaderType)]
pub struct EntityIndex {
    entity_index: u32,
}

impl EntityIndex {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity_index: entity.index(),
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct EntityIndexLayout {
    pub layout: BindGroupLayout,
}

impl FromWorld for EntityIndexLayout {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(EntityIndex::min_size()),
                },
                count: None,
            }],
            label: Some("entity_index_layout"),
        });

        EntityIndexLayout { layout }
    }
}

// #[derive(Debug, Resource, Deref, DerefMut, Default)]
// pub struct BatchVertexOffsets {
//     pub offset: HashMap<Entity, BindGroup>,
// }

// A single batch, e.g. `[SpriteBatch]` or `[UiBatch]` will have some offset into a storage buffer.
// When a draw call is issued, the starting vertex is tied to the range of the batch.
// E.g. if a draw call draws vertices (100..200), the starting vertex is 100.
// However the storage buffer bound is unique to the batch, so the starting vertex is 100 + the offset.
// #[derive(Debug, Resource, Deref, DerefMut)]
// pub struct BatchVertexOffsetsLayout {
//     pub layout: BindGroupLayout,
// }

// impl FromWorld for BatchVertexOffsetsLayout {
//     fn from_world(world: &mut World) -> Self {
//         let render_device = world.resource::<RenderDevice>();

//         let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
//             entries: &[BindGroupLayoutEntry {
//                 binding: 0,
//                 visibility: ShaderStages::VERTEX,
//                 ty: BindingType::Buffer {
//                     ty: BufferBindingType::Uniform,
//                     has_dynamic_offset: false,
//                     min_binding_size: Some(
//                         NonZeroU64::new(std::mem::size_of::<u32>() as u64).unwrap(),
//                     ),
//                 },
//                 count: None,
//             }],
//             label: Some("batch_offsets_layout"),
//         });

//         BatchVertexOffsetsLayout { layout }
//     }
// }

// Bind group insertion
// fn batch_item_add_range_start(
//     // mut commands: Commands,
//     device: Res<RenderDevice>,
//     queue: Res<RenderQueue>,
//     bind_group_layout: Res<BatchVertexOffsetsLayout>,
//     mut offsets: ResMut<BatchVertexOffsets>,
//     render_phases: Query<&RenderPhase<Transparent2d>>,
// ) {
//     offsets.clear();

//     for phase in &render_phases {
//         for item in &phase.items {
//             let Some(range) = item.batch_range().as_ref() else { continue };

//             let mut buffer = UniformBuffer::default();

//             buffer.set(range.start);
//             buffer.write_buffer(&device, &queue);

//             let bind_group = device.create_bind_group(&BindGroupDescriptor {
//                 label: Some("BatchVertexOffset"),
//                 layout: &bind_group_layout.layout,
//                 entries: &[BindGroupEntry {
//                     binding: 0,
//                     resource: buffer.binding().unwrap(),
//                 }],
//             });

//             offsets.insert(item.entity, bind_group);

//             // commands.entity(item.entity).insert(BatchVertexOffset {
//             //     offset: range.start,
//             // });
//         }
//     }
// }
