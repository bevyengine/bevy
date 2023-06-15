//! Bind group layout related definitions for the mesh pipeline.

use bevy_render::{
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, Buffer,
    },
    renderer::RenderDevice,
};

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

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts {
    /// The mesh model uniform (transform) and nothing else.
    pub model_only: BindGroupLayout,

    /// Also includes the uniform for skinning
    pub skinned: BindGroupLayout,
}
impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(render_device: &RenderDevice) -> Self {
        MeshLayouts {
            model_only: Self::model_only_layout(render_device),
            skinned: Self::skinned_layout(render_device),
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn model_only_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(0)],
            label: Some("mesh_layout"),
        })
    }
    fn skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(0), layout_entry::skinning(1)],
            label: Some("skinned_mesh_layout"),
        })
    }

    // ---------- BindGroup methods ----------

    pub fn model_only(&self, render_device: &RenderDevice, model: &Buffer) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            layout: &self.model_only,
            entries: &[entry::model(0, model)],
            label: Some("model_only_mesh_bind_group"),
        })
    }
    pub fn skinned(
        &self,
        render_device: &RenderDevice,
        model: &Buffer,
        skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            layout: &self.skinned,
            entries: &[entry::model(0, model), entry::skinning(1, skin)],
            label: Some("skinned_mesh_bind_group"),
        })
    }
}
