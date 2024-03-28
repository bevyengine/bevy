//! Logic for indirect rendering.
//!
//! Currently, indirect rendering is enabled whenever GPU culling is enabled.

use std::{any::TypeId, ops::Range};

use bevy_app::{App, Plugin};
use bevy_ecs::{
    entity::EntityHashMap,
    schedule::IntoSystemConfigs as _,
    system::{Res, ResMut, Resource},
};
use bevy_encase_derive::ShaderType;
use bevy_math::{Vec3, Vec3A};
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use crate::{
    primitives::Aabb,
    render_resource::{binding_types, BindGroupLayoutEntryBuilder, BufferVec},
    renderer::{RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};

pub struct IndirectRenderPlugin;

/// This covers both the indexed indirect and regular indirect parameters.
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct GpuIndirectParameters {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub extra_0: u32,
    pub extra_1: u32,
    pub first_instance: u32,
}

#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct GpuIndirectInstanceDescriptor {
    pub parameters_index: u32,
    pub instance_index: u32,
}

#[derive(Resource)]
pub struct IndirectBuffers {
    pub params: BufferVec<GpuIndirectParameters>,
    pub mesh_indirect_uniform: BufferVec<MeshIndirectUniform>,
    pub view_instances: EntityHashMap<ViewIndirectInstances>,
}

pub struct ViewIndirectInstances {
    pub instances: BufferVec<u32>,
    pub instance_count: u32,
    /// These represent the culling work units.
    pub descriptors: BufferVec<GpuIndirectInstanceDescriptor>,
    ///  The `TypeId` here is the type ID of a `PhaseItem`.
    pub phase_item_ranges: HashMap<TypeId, Range<u32>>,
}

#[derive(Clone)]
pub struct RenderMeshIndirectInstance {
    pub aabb: Aabb,
}

#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct MeshIndirectUniform {
    pub aabb_center: Vec3,
    pub pad0: u32,
    pub aabb_half_extents: Vec3,
    pub pad1: u32,
}

impl Plugin for IndirectRenderPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<IndirectBuffers>().add_systems(
            Render,
            write_indirect_buffers.in_set(RenderSet::PrepareResourcesFlush),
        );
    }
}

impl Default for IndirectBuffers {
    fn default() -> Self {
        IndirectBuffers {
            params: BufferVec::new(BufferUsages::STORAGE | BufferUsages::INDIRECT),
            view_instances: EntityHashMap::default(),
            mesh_indirect_uniform: BufferVec::new(BufferUsages::STORAGE),
        }
    }
}

impl ViewIndirectInstances {
    pub fn new() -> ViewIndirectInstances {
        ViewIndirectInstances {
            instances: BufferVec::new(BufferUsages::STORAGE),
            instance_count: 0,
            descriptors: BufferVec::new(BufferUsages::STORAGE),
            phase_item_ranges: HashMap::new(),
        }
    }
}

impl Default for ViewIndirectInstances {
    fn default() -> Self {
        Self::new()
    }
}

pub fn write_indirect_buffers(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut indirect_buffers: ResMut<IndirectBuffers>,
) {
    indirect_buffers
        .params
        .write_buffer(&render_device, &render_queue);
    indirect_buffers
        .mesh_indirect_uniform
        .write_buffer(&render_device, &render_queue);

    for view_instance in indirect_buffers.view_instances.values_mut() {
        view_instance
            .instances
            .reserve(view_instance.instance_count as usize, &render_device);
        view_instance
            .descriptors
            .write_buffer(&render_device, &render_queue);
    }
}

pub fn get_bind_group_layout_entry(read_only: bool) -> BindGroupLayoutEntryBuilder {
    if read_only {
        binding_types::storage_buffer_read_only::<u32>(/*has_dynamic_offset=*/ false)
    } else {
        binding_types::storage_buffer::<u32>(/*has_dynamic_offset=*/ false)
    }
}
