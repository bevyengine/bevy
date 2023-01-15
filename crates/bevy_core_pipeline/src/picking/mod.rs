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
            .init_resource::<EntityIndexLayout>()
            .add_system_to_stage(RenderStage::Prepare, picking::prepare_picking_targets);

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
