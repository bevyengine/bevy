//! Bind group layout related definitions for the mesh pipeline.

use bevy_render::{
    mesh::morph::MAX_MORPH_WEIGHTS,
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, Buffer,
        TextureView,
    },
    renderer::RenderDevice,
};

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

/// Individual layout entries.
mod layout_entry {
    use super::MORPH_BUFFER_SIZE;
    use crate::render::mesh::JOINT_BUFFER_SIZE;
    use crate::MeshUniform;
    use bevy_render::render_resource::{
        BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, ShaderStages, ShaderType,
        TextureSampleType, TextureViewDimension,
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
    pub(super) fn weights(binding: u32) -> BindGroupLayoutEntry {
        buffer(binding, MORPH_BUFFER_SIZE as u64, ShaderStages::VERTEX)
    }
    pub(super) fn targets(binding: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Texture {
                view_dimension: TextureViewDimension::D3,
                sample_type: TextureSampleType::Float { filterable: false },
                multisampled: false,
            },
            count: None,
        }
    }
}
/// Individual [`BindGroupEntry`](bevy_render::render_resource::BindGroupEntry)
/// for bind groups.
mod entry {
    use super::MORPH_BUFFER_SIZE;
    use crate::render::mesh::JOINT_BUFFER_SIZE;
    use crate::MeshUniform;
    use bevy_render::render_resource::{
        BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, ShaderType, TextureView,
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
    pub(super) fn weights(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, MORPH_BUFFER_SIZE as u64, buffer)
    }
    pub(super) fn targets(binding: u32, texture: &TextureView) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::TextureView(texture),
        }
    }
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts {
    /// The mesh model uniform (transform) and nothing else.
    pub model_only: BindGroupLayout,

    /// Also includes the uniform for skinning
    pub skinned: BindGroupLayout,

    /// Also includes the uniform and [`MorphAttributes`] for morph targets.
    ///
    /// [`MorphAttributes`]: bevy_render::mesh::morph::MorphAttributes
    pub morphed: BindGroupLayout,

    /// Also includes both uniforms for skinning and morph targets, also the
    /// morph target [`MorphAttributes`] binding.
    ///
    /// [`MorphAttributes`]: bevy_render::mesh::morph::MorphAttributes
    pub morphed_skinned: BindGroupLayout,
}

impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(render_device: &RenderDevice) -> Self {
        MeshLayouts {
            model_only: Self::model_only_layout(render_device),
            skinned: Self::skinned_layout(render_device),
            morphed: Self::morphed_layout(render_device),
            morphed_skinned: Self::morphed_skinned_layout(render_device),
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
    fn morphed_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(0),
                layout_entry::weights(2),
                layout_entry::targets(3),
            ],
            label: Some("morphed_mesh_layout"),
        })
    }
    fn morphed_skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(0),
                layout_entry::skinning(1),
                layout_entry::weights(2),
                layout_entry::targets(3),
            ],
            label: Some("morphed_skinned_mesh_layout"),
        })
    }

    // ---------- BindGroup methods ----------

    pub fn model_only(&self, render_device: &RenderDevice, model: &Buffer) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::model(0, model)],
            layout: &self.model_only,
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
            entries: &[entry::model(0, model), entry::skinning(1, skin)],
            layout: &self.skinned,
            label: Some("skinned_mesh_bind_group"),
        })
    }
    pub fn morphed(
        &self,
        render_device: &RenderDevice,
        model: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::model(0, model),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
            layout: &self.morphed,
            label: Some("morphed_mesh_bind_group"),
        })
    }
    pub fn morphed_skinned(
        &self,
        render_device: &RenderDevice,
        model: &Buffer,
        skin: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::model(0, model),
                entry::skinning(1, skin),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
            layout: &self.morphed_skinned,
            label: Some("morphed_skinned_mesh_bind_group"),
        })
    }
}
