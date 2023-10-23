//! Bind group layout related definitions for the mesh pipeline.

use bevy_math::Mat4;
use bevy_render::{
    mesh::morph::MAX_MORPH_WEIGHTS,
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindingResource, Buffer, TextureView,
    },
    renderer::RenderDevice,
    texture::GpuImage,
};

use crate::{render::skin::MAX_JOINTS, GpuLightmap, MAX_LIGHTMAPS};

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

const LIGHTMAP_SIZE: usize = std::mem::size_of::<GpuLightmap>();
pub const LIGHTMAP_BUFFER_SIZE: usize = MAX_LIGHTMAPS * LIGHTMAP_SIZE;

/// Individual layout entries.
mod layout_entry {
    use super::{JOINT_BUFFER_SIZE, LIGHTMAP_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use crate::MeshUniform;
    use bevy_render::{
        render_resource::{
            BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, GpuArrayBuffer,
            SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
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
    pub(super) fn model(render_device: &RenderDevice, binding: u32) -> BindGroupLayoutEntry {
        GpuArrayBuffer::<MeshUniform>::binding_layout(
            binding,
            ShaderStages::VERTEX_FRAGMENT,
            render_device,
        )
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
    pub(super) fn lightmaps_texture_view(binding: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2Array,
                multisampled: false,
            },
            count: None,
        }
    }
    pub(super) fn lightmaps_sampler(binding: u32) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        }
    }
    pub(super) fn lightmaps(binding: u32) -> BindGroupLayoutEntry {
        // This one doesn't use a dynamic offset, because the offset is in the
        // Mesh structure.
        BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: BufferSize::new(LIGHTMAP_BUFFER_SIZE as u64),
            },
        }
    }
}
/// Individual [`BindGroupEntry`](bevy_render::render_resource::BindGroupEntry)
/// for bind groups.
mod entry {
    use super::{JOINT_BUFFER_SIZE, LIGHTMAP_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use bevy_render::render_resource::{
        BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, Sampler, TextureView,
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
    pub(super) fn model(binding: u32, resource: BindingResource) -> BindGroupEntry {
        BindGroupEntry { binding, resource }
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
    pub(super) fn lightmaps_texture_view(binding: u32, texture: &TextureView) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::TextureView(texture),
        }
    }
    pub(super) fn lightmaps_sampler(binding: u32, sampler: &Sampler) -> BindGroupEntry {
        BindGroupEntry {
            binding,
            resource: BindingResource::Sampler(sampler),
        }
    }
    pub(super) fn lightmaps(binding: u32, buffer: &Buffer) -> BindGroupEntry {
        entry(binding, LIGHTMAP_BUFFER_SIZE as u64, buffer)
    }
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts {
    /// The mesh model uniform (transform) and nothing else.
    pub model_only: BindGroupLayout,

    /// Includes the lightmap texture and uniform.
    pub lightmapped: BindGroupLayout,

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
            lightmapped: Self::lightmapped_layout(render_device),
            skinned: Self::skinned_layout(render_device),
            morphed: Self::morphed_layout(render_device),
            morphed_skinned: Self::morphed_skinned_layout(render_device),
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn model_only_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(render_device, 0)],
            label: Some("mesh_layout"),
        })
    }
    fn lightmapped_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::lightmaps_texture_view(4),
                layout_entry::lightmaps_sampler(5),
                layout_entry::lightmaps(6),
            ],
            label: Some("lightmapped_mesh_layout"),
        })
    }
    fn skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::skinning(1),
            ],
            label: Some("skinned_mesh_layout"),
        })
    }
    fn morphed_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::weights(2),
                layout_entry::targets(3),
            ],
            label: Some("morphed_mesh_layout"),
        })
    }
    fn morphed_skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::skinning(1),
                layout_entry::weights(2),
                layout_entry::targets(3),
            ],
            label: Some("morphed_skinned_mesh_layout"),
        })
    }

    // ---------- BindGroup methods ----------

    pub fn model_only(&self, render_device: &RenderDevice, model: &BindingResource) -> BindGroup {
        render_device.create_bind_group(
            "model_only_mesh_bind_group",
            &self.model_only,
            &[entry::model(0, model.clone())],
        )
    }
    pub fn lightmapped(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        lightmap_image: &GpuImage,
        lightmap_uniform: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "lightmapped_mesh_bind_group",
            &self.lightmapped,
            &[
                entry::model(0, model.clone()),
                entry::lightmaps_texture_view(4, &lightmap_image.texture_view),
                entry::lightmaps_sampler(5, &lightmap_image.sampler),
                entry::lightmaps(6, lightmap_uniform),
            ],
        )
    }
    pub fn skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "skinned_mesh_bind_group",
            &self.skinned,
            &[entry::model(0, model.clone()), entry::skinning(1, skin)],
        )
    }
    pub fn morphed(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(
            "morphed_mesh_bind_group",
            &self.morphed,
            &[
                entry::model(0, model.clone()),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
        )
    }
    pub fn morphed_skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(
            "morphed_skinned_mesh_bind_group",
            &self.morphed_skinned,
            &[
                entry::model(0, model.clone()),
                entry::skinning(1, skin),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
        )
    }
}
