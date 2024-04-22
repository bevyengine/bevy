//! Bind group layout related definitions for the mesh pipeline.

use bevy_math::Mat4;
use bevy_render::{
    mesh::morph::MAX_MORPH_WEIGHTS, render_resource::*, renderer::RenderDevice, texture::GpuImage,
};

use crate::render::skin::MAX_JOINTS;

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

/// Individual layout entries.
mod layout_entry {
    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use crate::MeshUniform;
    use bevy_render::{
        render_resource::{
            binding_types::{sampler, texture_2d, texture_3d, uniform_buffer_sized},
            BindGroupLayoutEntryBuilder, BufferSize, GpuArrayBuffer, SamplerBindingType,
            ShaderStages, TextureSampleType,
        },
        renderer::RenderDevice,
    };

    pub(super) fn model(render_device: &RenderDevice) -> BindGroupLayoutEntryBuilder {
        GpuArrayBuffer::<MeshUniform>::binding_layout(render_device)
            .visibility(ShaderStages::VERTEX_FRAGMENT)
    }
    pub(super) fn skinning() -> BindGroupLayoutEntryBuilder {
        uniform_buffer_sized(true, BufferSize::new(JOINT_BUFFER_SIZE as u64))
    }
    pub(super) fn weights() -> BindGroupLayoutEntryBuilder {
        uniform_buffer_sized(true, BufferSize::new(MORPH_BUFFER_SIZE as u64))
    }
    pub(super) fn targets() -> BindGroupLayoutEntryBuilder {
        texture_3d(TextureSampleType::Float { filterable: false })
    }
    pub(super) fn lightmaps_texture_view() -> BindGroupLayoutEntryBuilder {
        texture_2d(TextureSampleType::Float { filterable: true }).visibility(ShaderStages::FRAGMENT)
    }
    pub(super) fn lightmaps_sampler() -> BindGroupLayoutEntryBuilder {
        sampler(SamplerBindingType::Filtering).visibility(ShaderStages::FRAGMENT)
    }
}

/// Individual [`BindGroupEntry`]
/// for bind groups.
mod entry {
    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
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

    /// TODO
    pub skinned_motion_vectors: Option<BindGroupLayout>,

    /// TODO
    pub morphed_motion_vectors: Option<BindGroupLayout>,

    /// TODO
    pub morphed_skinned_motion_vectors: Option<BindGroupLayout>,
}

impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(render_device: &RenderDevice, motion_vector_support: bool) -> Self {
        MeshLayouts {
            model_only: Self::model_only_layout(render_device),
            lightmapped: Self::lightmapped_layout(render_device),
            skinned: Self::skinned_layout(render_device),
            morphed: Self::morphed_layout(render_device),
            morphed_skinned: Self::morphed_skinned_layout(render_device),
            skinned_motion_vectors: motion_vector_support
                .then(|| Self::skinned_motion_vectors_layout(render_device)),
            morphed_motion_vectors: motion_vector_support
                .then(|| Self::morphed_motion_vectors_layout(render_device)),
            morphed_skinned_motion_vectors: motion_vector_support
                .then(|| Self::morphed_skinned_motion_vectors_layout(render_device)),
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn model_only_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "mesh_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::empty(),
                layout_entry::model(render_device),
            ),
        )
    }
    fn lightmapped_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "lightmapped_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (4, layout_entry::lightmaps_texture_view()),
                    (5, layout_entry::lightmaps_sampler()),
                ),
            ),
        )
    }
    fn skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "skinned_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (1, layout_entry::skinning()),
                ),
            ),
        )
    }
    fn morphed_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "morphed_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (2, layout_entry::weights()),
                    (3, layout_entry::targets()),
                ),
            ),
        )
    }
    fn morphed_skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "morphed_skinned_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (1, layout_entry::skinning()),
                    (2, layout_entry::weights()),
                    (3, layout_entry::targets()),
                ),
            ),
        )
    }
    fn skinned_motion_vectors_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "skinned_motion_vectors_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (1, layout_entry::skinning()),
                    (6, layout_entry::skinning()),
                ),
            ),
        )
    }
    fn morphed_motion_vectors_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "morphed_motion_vectors_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (2, layout_entry::weights()),
                    (3, layout_entry::targets()),
                    (7, layout_entry::weights()),
                ),
            ),
        )
    }
    fn morphed_skinned_motion_vectors_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(
            "morphed_skinned_motion_vectors_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(render_device)),
                    (1, layout_entry::skinning()),
                    (2, layout_entry::weights()),
                    (3, layout_entry::targets()),
                    (6, layout_entry::skinning()),
                    (7, layout_entry::weights()),
                ),
            ),
        )
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
        lightmap: &GpuImage,
    ) -> BindGroup {
        render_device.create_bind_group(
            "lightmapped_mesh_bind_group",
            &self.lightmapped,
            &[
                entry::model(0, model.clone()),
                entry::lightmaps_texture_view(4, &lightmap.texture_view),
                entry::lightmaps_sampler(5, &lightmap.sampler),
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
    pub fn skinned_motion_vectors(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        previous_skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "skinned_motion_vectors_mesh_bind_group",
            self.skinned_motion_vectors.as_ref().unwrap(),
            &[
                entry::model(0, model.clone()),
                entry::skinning(1, skin),
                entry::skinning(6, previous_skin),
            ],
        )
    }
    pub fn morphed_motion_vectors(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        weights: &Buffer,
        targets: &TextureView,
        previous_weights: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "morphed_motion_vectors_mesh_bind_group",
            self.morphed_motion_vectors.as_ref().unwrap(),
            &[
                entry::model(0, model.clone()),
                entry::weights(2, weights),
                entry::targets(3, targets),
                entry::weights(7, previous_weights),
            ],
        )
    }
    pub fn morphed_skinned_motion_vectors(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
        previous_skin: &Buffer,
        previous_weights: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "morphed_skinned_motion_vectors_mesh_bind_group",
            self.morphed_skinned_motion_vectors.as_ref().unwrap(),
            &[
                entry::model(0, model.clone()),
                entry::skinning(1, skin),
                entry::weights(2, weights),
                entry::targets(3, targets),
                entry::skinning(6, previous_skin),
                entry::weights(7, previous_weights),
            ],
        )
    }
}
