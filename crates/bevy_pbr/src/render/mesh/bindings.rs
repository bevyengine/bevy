//! Bind [`group`] layout related definitions for the mesh pipeline.

/// Individual [`layout`] entries.
mod layout_entry {
    use crate::render::mesh::JOINT_BUFFER_SIZE;
    use crate::MeshUniform;
    use bevy_render::render_resource::{
        BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, ShaderStages, ShaderType,
    };

    fn buffer(binding: u32, size: u64, visibility: ShaderStages) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: BufferSize::new(size),
            },
        }
    }
    pub(super) fn model(binding: u32) -> BindGroupLayoutEntry {
        let size = MeshUniform::min_size().get();
        buffer(binding, size, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
    }
    pub(super) fn skinning(binding: u32) -> BindGroupLayoutEntry {
        buffer(binding, JOINT_BUFFER_SIZE as u64, ShaderStages::VERTEX)
    }
}
/// [`BindGroupLayout`](bevy_render::render_resource::BindGroupLayout)s.
pub mod layout {
    use bevy_render::{
        render_resource::{BindGroupLayout, BindGroupLayoutDescriptor},
        renderer::RenderDevice,
    };

    use super::layout_entry;

    pub fn model_only(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(0)],
            label: Some("mesh_layout"),
        })
    }
    pub fn skinned(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(0), layout_entry::skinning(1)],
            label: Some("skinned_mesh_layout"),
        })
    }
}
/// Individual [`BindGroupEntry`](bevy_render::render_resource::BindGroupEntry)
/// for bind [`group`]s.
mod entry {
    use crate::render::mesh::JOINT_BUFFER_SIZE;
    use crate::MeshUniform;
    use bevy_render::render_resource::{
        BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, ShaderType,
    };

    fn entry(binding: u32, size: u64, buffer: &Buffer) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset: 0,
                size: Some(BufferSize::new(size).unwrap()),
            }),
        }
    }
    pub(super) fn model(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, MeshUniform::min_size().get(), buffer)
    }
    pub(super) fn skinning(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, JOINT_BUFFER_SIZE as u64, buffer)
    }
}
/// [`BindGroup`](bevy_render::render_resource::BindGroup)s.
pub mod group {
    use bevy_render::{
        render_resource::{BindGroup, BindGroupDescriptor, BindGroupLayout, Buffer},
        renderer::RenderDevice,
    };

    use super::entry;

    pub fn model_only(
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
        model: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::model(0, model)],
            layout,
            label: Some("model_only_mesh_bind_group"),
        })
    }
    pub fn skinned(
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
        model: &Buffer,
        skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::model(0, model), entry::skinning(1, skin)],
            layout,
            label: Some("skinned_mesh_bind_group"),
        })
    }
}
