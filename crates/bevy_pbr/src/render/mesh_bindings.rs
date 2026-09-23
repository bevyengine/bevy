//! Bind group layout related definitions for the mesh pipeline.

use arrayvec::ArrayVec;
use bevy_math::Mat4;
use bevy_mesh::morph::MAX_MORPH_WEIGHTS;
use bevy_render::{
    mesh::morph::MorphTargetsResource,
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice},
};

use crate::{binding_arrays_are_usable, render::skin::MAX_JOINTS, skin, LightmapSlab};

const MORPH_WEIGHT_SIZE: usize = size_of::<f32>();

/// This is used to allocate buffers.
/// The correctness of the value depends on the GPU/platform.
/// The current value is chosen because it is guaranteed to work everywhere.
/// To allow for bigger values, a check must be made for the limits
/// of the GPU at runtime, which would mean not using consts anymore.
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

const JOINT_SIZE: usize = size_of::<Mat4>();
pub(crate) const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

/// Individual layout entries.
mod layout_entry {
    use core::num::NonZeroU32;

    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use crate::{render::skin, GpuMorphDescriptor, MeshUniform, LIGHTMAPS_PER_SLAB};
    use bevy_mesh::morph::MorphAttributes;
    use bevy_render::{
        mesh::MeshMetadata,
        render_resource::{
            binding_types::{
                sampler, storage_buffer_read_only, storage_buffer_read_only_sized, texture_2d,
                texture_3d, uniform_buffer_sized,
            },
            BindGroupLayoutEntryBuilder, BufferSize, GpuArrayBuffer, SamplerBindingType,
            ShaderStages, TextureSampleType,
        },
        settings::WgpuLimits,
    };

    pub(super) fn model(limits: &WgpuLimits) -> BindGroupLayoutEntryBuilder {
        GpuArrayBuffer::<MeshUniform>::binding_layout(limits)
            .visibility(ShaderStages::VERTEX_FRAGMENT)
    }
    pub(super) fn metadata(limits: &WgpuLimits) -> BindGroupLayoutEntryBuilder {
        if bevy_render::storage_buffers_are_unsupported(limits) {
            uniform_buffer_sized(false, BufferSize::new(size_of::<MeshMetadata>() as u64))
        } else {
            storage_buffer_read_only::<MeshMetadata>(false)
        }
    }
    pub(super) fn skinning(limits: &WgpuLimits) -> BindGroupLayoutEntryBuilder {
        // If we can use storage buffers, do so. Otherwise, fall back to uniform
        // buffers.
        let size = BufferSize::new(JOINT_BUFFER_SIZE as u64);
        if skin::skins_use_uniform_buffers(limits) {
            uniform_buffer_sized(true, size)
        } else {
            storage_buffer_read_only_sized(false, size)
        }
    }
    pub(super) fn weights(limits: &WgpuLimits) -> BindGroupLayoutEntryBuilder {
        if skin::skins_use_uniform_buffers(limits) {
            uniform_buffer_sized(true, BufferSize::new(MORPH_BUFFER_SIZE as u64))
        } else {
            storage_buffer_read_only::<f32>(false)
        }
    }
    pub(super) fn targets(limits: &WgpuLimits) -> BindGroupLayoutEntryBuilder {
        if skin::skins_use_uniform_buffers(limits) {
            texture_3d(TextureSampleType::Float { filterable: false })
        } else {
            storage_buffer_read_only::<MorphAttributes>(false)
        }
    }
    pub(super) fn morph_descriptors() -> BindGroupLayoutEntryBuilder {
        storage_buffer_read_only::<GpuMorphDescriptor>(false)
    }
    pub(super) fn lightmaps_texture_view() -> BindGroupLayoutEntryBuilder {
        texture_2d(TextureSampleType::Float { filterable: true }).visibility(ShaderStages::FRAGMENT)
    }
    pub(super) fn lightmaps_sampler() -> BindGroupLayoutEntryBuilder {
        sampler(SamplerBindingType::Filtering).visibility(ShaderStages::FRAGMENT)
    }
    pub(super) fn lightmaps_texture_view_array() -> BindGroupLayoutEntryBuilder {
        texture_2d(TextureSampleType::Float { filterable: true })
            .visibility(ShaderStages::FRAGMENT)
            .count(NonZeroU32::new(LIGHTMAPS_PER_SLAB as u32).unwrap())
    }
    pub(super) fn lightmaps_sampler_array() -> BindGroupLayoutEntryBuilder {
        sampler(SamplerBindingType::Filtering)
            .visibility(ShaderStages::FRAGMENT)
            .count(NonZeroU32::new(LIGHTMAPS_PER_SLAB as u32).unwrap())
    }
}

/// Individual [`BindGroupEntry`]
/// for bind groups.
mod entry {
    use crate::render::skin;
    use bevy_render::mesh::morph::MorphTargetsResource;

    use super::{JOINT_BUFFER_SIZE, MORPH_BUFFER_SIZE};
    use bevy_render::{
        render_resource::{
            BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, Sampler,
            TextureView, WgpuSampler, WgpuTextureView,
        },
        renderer::RenderDevice,
    };

    fn entry(binding: u32, size: Option<u64>, buffer: &Buffer) -> BindGroupEntry<'_> {
        BindGroupEntry {
            binding,
            resource: BindingResource::Buffer(BufferBinding {
                buffer,
                offset: 0,
                size: size.map(|size| BufferSize::new(size).unwrap()),
            }),
        }
    }
    pub(super) fn model(binding: u32, resource: BindingResource) -> BindGroupEntry {
        BindGroupEntry { binding, resource }
    }
    pub(super) fn metadata<'a>(binding: u32, buffer: &'a Buffer) -> BindGroupEntry<'a> {
        entry(binding, None, buffer)
    }
    pub(super) fn skinning<'a>(
        render_device: &RenderDevice,
        binding: u32,
        buffer: &'a Buffer,
    ) -> BindGroupEntry<'a> {
        let size = if skin::skins_use_uniform_buffers(&render_device.limits()) {
            Some(JOINT_BUFFER_SIZE as u64)
        } else {
            None
        };
        entry(binding, size, buffer)
    }
    pub(super) fn weights<'a>(
        render_device: &'_ RenderDevice,
        binding: u32,
        buffer: &'a Buffer,
    ) -> BindGroupEntry<'a> {
        if skin::skins_use_uniform_buffers(&render_device.limits()) {
            entry(binding, Some(MORPH_BUFFER_SIZE as u64), buffer)
        } else {
            entry(binding, None, buffer)
        }
    }
    pub(super) fn targets(binding: u32, targets: MorphTargetsResource<'_>) -> BindGroupEntry<'_> {
        BindGroupEntry {
            binding,
            resource: match targets {
                MorphTargetsResource::Texture(texture_view) => {
                    BindingResource::TextureView(texture_view)
                }
                MorphTargetsResource::Storage(buffer) => buffer.as_entire_binding(),
            },
        }
    }
    pub(super) fn morph_descriptors(binding: u32, buffer: &Buffer) -> BindGroupEntry<'_> {
        entry(binding, None, buffer)
    }
    pub(super) fn lightmaps_texture_view(
        binding: u32,
        texture: &TextureView,
    ) -> BindGroupEntry<'_> {
        BindGroupEntry {
            binding,
            resource: BindingResource::TextureView(texture),
        }
    }
    pub(super) fn lightmaps_sampler(binding: u32, sampler: &Sampler) -> BindGroupEntry<'_> {
        BindGroupEntry {
            binding,
            resource: BindingResource::Sampler(sampler),
        }
    }
    pub(super) fn lightmaps_texture_view_array<'a>(
        binding: u32,
        textures: &'a [&'a WgpuTextureView],
    ) -> BindGroupEntry<'a> {
        BindGroupEntry {
            binding,
            resource: BindingResource::TextureViewArray(textures),
        }
    }
    pub(super) fn lightmaps_sampler_array<'a>(
        binding: u32,
        samplers: &'a [&'a WgpuSampler],
    ) -> BindGroupEntry<'a> {
        BindGroupEntry {
            binding,
            resource: BindingResource::SamplerArray(samplers),
        }
    }
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wesl`).
#[derive(Clone)]
pub struct MeshLayouts {
    /// The mesh model uniform (transform) and nothing else.
    pub model_only: BindGroupLayoutDescriptor,

    /// Includes the lightmap texture and uniform.
    pub lightmapped: BindGroupLayoutDescriptor,

    /// Also includes the uniform for skinning
    pub skinned: BindGroupLayoutDescriptor,

    /// Like [`MeshLayouts::skinned`], but includes slots for the previous
    /// frame's joint matrices, so that we can compute motion vectors.
    pub skinned_motion: BindGroupLayoutDescriptor,

    /// Also includes the uniform and [`MorphAttributes`] for morph targets.
    ///
    /// [`MorphAttributes`]: bevy_mesh::morph::MorphAttributes
    pub morphed: BindGroupLayoutDescriptor,

    /// Like [`MeshLayouts::morphed`], but includes a slot for the previous
    /// frame's morph weights, so that we can compute motion vectors.
    pub morphed_motion: BindGroupLayoutDescriptor,

    /// Also includes both uniforms for skinning and morph targets, also the
    /// morph target [`MorphAttributes`] binding.
    ///
    /// [`MorphAttributes`]: bevy_mesh::morph::MorphAttributes
    pub morphed_skinned: BindGroupLayoutDescriptor,

    /// Like [`MeshLayouts::morphed_skinned`], but includes slots for the
    /// previous frame's joint matrices and morph weights, so that we can
    /// compute motion vectors.
    pub morphed_skinned_motion: BindGroupLayoutDescriptor,

    /// The descriptors above resolved to GPU bind group layouts once, so that
    /// creating a bind group each frame does not look its layout up in the
    /// pipeline cache (a hash of the whole descriptor under a lock) again.
    bind_group_layouts: MeshBindGroupLayouts,
}

/// The [`BindGroupLayout`]s of a [`MeshLayouts`]; see
/// [`MeshLayouts::bind_group_layouts`].
#[derive(Clone)]
struct MeshBindGroupLayouts {
    model_only: BindGroupLayout,
    lightmapped: BindGroupLayout,
    skinned: BindGroupLayout,
    skinned_motion: BindGroupLayout,
    morphed: BindGroupLayout,
    morphed_motion: BindGroupLayout,
    morphed_skinned: BindGroupLayout,
    morphed_skinned_motion: BindGroupLayout,
}

impl MeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_mesh::Mesh
    pub fn new(
        render_device: &RenderDevice,
        render_adapter: &RenderAdapter,
        pipeline_cache: &PipelineCache,
    ) -> Self {
        let model_only = Self::model_only_layout(render_device);
        let lightmapped = Self::lightmapped_layout(render_device, render_adapter);
        let skinned = Self::skinned_layout(render_device);
        let skinned_motion = Self::skinned_motion_layout(render_device);
        let morphed = Self::morphed_layout(render_device);
        let morphed_motion = Self::morphed_motion_layout(render_device);
        let morphed_skinned = Self::morphed_skinned_layout(render_device);
        let morphed_skinned_motion = Self::morphed_skinned_motion_layout(render_device);
        let bind_group_layouts = MeshBindGroupLayouts {
            model_only: pipeline_cache.get_bind_group_layout(&model_only),
            lightmapped: pipeline_cache.get_bind_group_layout(&lightmapped),
            skinned: pipeline_cache.get_bind_group_layout(&skinned),
            skinned_motion: pipeline_cache.get_bind_group_layout(&skinned_motion),
            morphed: pipeline_cache.get_bind_group_layout(&morphed),
            morphed_motion: pipeline_cache.get_bind_group_layout(&morphed_motion),
            morphed_skinned: pipeline_cache.get_bind_group_layout(&morphed_skinned),
            morphed_skinned_motion: pipeline_cache.get_bind_group_layout(&morphed_skinned_motion),
        };
        MeshLayouts {
            model_only,
            lightmapped,
            skinned,
            skinned_motion,
            morphed,
            morphed_motion,
            morphed_skinned,
            morphed_skinned_motion,
            bind_group_layouts,
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn model_only_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        BindGroupLayoutDescriptor::new(
            "mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(&limits)),
                    (9, layout_entry::metadata(&limits)),
                ),
            ),
        )
    }

    /// Creates the layout for skinned meshes.
    fn skinned_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        BindGroupLayoutDescriptor::new(
            "skinned_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(&limits)),
                    (9, layout_entry::metadata(&limits)),
                    // The current frame's joint matrix buffer.
                    (1, layout_entry::skinning(&limits)),
                ),
            ),
        )
    }

    /// Creates the layout for skinned meshes with the infrastructure to compute
    /// motion vectors.
    fn skinned_motion_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        BindGroupLayoutDescriptor::new(
            "skinned_motion_mesh_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX,
                (
                    (0, layout_entry::model(&limits)),
                    (9, layout_entry::metadata(&limits)),
                    // The current frame's joint matrix buffer.
                    (1, layout_entry::skinning(&limits)),
                    // The previous frame's joint matrix buffer.
                    (6, layout_entry::skinning(&limits)),
                ),
            ),
        )
    }

    /// Creates the layout for meshes with morph targets.
    fn morphed_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        let mut entries: ArrayVec<BindGroupLayoutEntry, 5> = ArrayVec::new();

        entries.extend(
            [
                (0, layout_entry::model(&limits)),
                (9, layout_entry::metadata(&limits)),
                // The current frame's morph weight buffer.
                (2, layout_entry::weights(&limits)),
                (3, layout_entry::targets(&limits)),
            ]
            .iter()
            .map(|(binding, entry)| entry.build(*binding, ShaderStages::VERTEX)),
        );

        if !skin::skins_use_uniform_buffers(&limits) {
            entries.push(layout_entry::morph_descriptors().build(8, ShaderStages::VERTEX));
        }

        BindGroupLayoutDescriptor::new("morphed_mesh_layout", &entries)
    }

    /// Creates the layout for meshes with morph targets and the infrastructure
    /// to compute motion vectors.
    fn morphed_motion_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        let mut entries: ArrayVec<BindGroupLayoutEntry, 6> = ArrayVec::new();

        entries.extend(
            [
                (0, layout_entry::model(&limits)),
                (9, layout_entry::metadata(&limits)),
                // The current frame's morph weight buffer.
                (2, layout_entry::weights(&limits)),
                (3, layout_entry::targets(&limits)),
                // The previous frame's morph weight buffer.
                (7, layout_entry::weights(&limits)),
            ]
            .iter()
            .map(|(binding, entry)| entry.build(*binding, ShaderStages::VERTEX)),
        );

        if !skin::skins_use_uniform_buffers(&limits) {
            entries.push(layout_entry::morph_descriptors().build(8, ShaderStages::VERTEX));
        }

        BindGroupLayoutDescriptor::new("morphed_motion_layout", &entries)
    }

    /// Creates the bind group layout for meshes with both skins and morph
    /// targets.
    fn morphed_skinned_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();

        let mut entries: ArrayVec<BindGroupLayoutEntry, 6> = ArrayVec::new();

        entries.extend(
            [
                (0, layout_entry::model(&limits)),
                (9, layout_entry::metadata(&limits)),
                // The current frame's joint matrix buffer.
                (1, layout_entry::skinning(&limits)),
                // The current frame's morph weight buffer.
                (2, layout_entry::weights(&limits)),
                (3, layout_entry::targets(&limits)),
            ]
            .iter()
            .map(|(binding, entry)| entry.build(*binding, ShaderStages::VERTEX)),
        );

        if !skin::skins_use_uniform_buffers(&limits) {
            entries.push(layout_entry::morph_descriptors().build(8, ShaderStages::VERTEX));
        }

        BindGroupLayoutDescriptor::new("morphed_skinned_mesh_layout", &entries)
    }

    /// Creates the bind group layout for meshes with both skins and morph
    /// targets, in addition to the infrastructure to compute motion vectors.
    fn morphed_skinned_motion_layout(render_device: &RenderDevice) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();

        let mut entries: ArrayVec<BindGroupLayoutEntry, 8> = ArrayVec::new();

        entries.extend(
            [
                (0, layout_entry::model(&limits)),
                (9, layout_entry::metadata(&limits)),
                // The current frame's joint matrix buffer.
                (1, layout_entry::skinning(&limits)),
                // The current frame's morph weight buffer.
                (2, layout_entry::weights(&limits)),
                (3, layout_entry::targets(&limits)),
                // The previous frame's joint matrix buffer.
                (6, layout_entry::skinning(&limits)),
                // The previous frame's morph weight buffer.
                (7, layout_entry::weights(&limits)),
            ]
            .iter()
            .map(|(binding, entry)| entry.build(*binding, ShaderStages::VERTEX)),
        );

        if !skin::skins_use_uniform_buffers(&limits) {
            entries.push(layout_entry::morph_descriptors().build(8, ShaderStages::VERTEX));
        }

        BindGroupLayoutDescriptor::new("morphed_skinned_motion_mesh_layout", &entries)
    }

    fn lightmapped_layout(
        render_device: &RenderDevice,
        render_adapter: &RenderAdapter,
    ) -> BindGroupLayoutDescriptor {
        let limits = render_device.limits();
        if binding_arrays_are_usable(render_device, render_adapter) {
            BindGroupLayoutDescriptor::new(
                "lightmapped_mesh_layout",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX,
                    (
                        (0, layout_entry::model(&limits)),
                        (9, layout_entry::metadata(&limits)),
                        (4, layout_entry::lightmaps_texture_view_array()),
                        (5, layout_entry::lightmaps_sampler_array()),
                    ),
                ),
            )
        } else {
            BindGroupLayoutDescriptor::new(
                "lightmapped_mesh_layout",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX,
                    (
                        (0, layout_entry::model(&limits)),
                        (9, layout_entry::metadata(&limits)),
                        (4, layout_entry::lightmaps_texture_view()),
                        (5, layout_entry::lightmaps_sampler()),
                    ),
                ),
            )
        }
    }

    // ---------- BindGroup methods ----------

    pub fn model_only(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "model_only_mesh_bind_group",
            &self.bind_group_layouts.model_only,
            &[entry::model(0, model.clone()), entry::metadata(9, metadata)],
        )
    }

    pub fn lightmapped(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        lightmap_slab: &LightmapSlab,
        bindless_lightmaps: bool,
    ) -> BindGroup {
        if bindless_lightmaps {
            let (texture_views, samplers) = lightmap_slab.build_binding_arrays();
            render_device.create_bind_group(
                "lightmapped_mesh_bind_group",
                &self.bind_group_layouts.lightmapped,
                &[
                    entry::model(0, model.clone()),
                    entry::metadata(9, metadata),
                    entry::lightmaps_texture_view_array(4, &texture_views),
                    entry::lightmaps_sampler_array(5, &samplers),
                ],
            )
        } else {
            let (texture_view, sampler) = lightmap_slab.bindings_for_first_lightmap();
            render_device.create_bind_group(
                "lightmapped_mesh_bind_group",
                &self.bind_group_layouts.lightmapped,
                &[
                    entry::model(0, model.clone()),
                    entry::metadata(9, metadata),
                    entry::lightmaps_texture_view(4, texture_view),
                    entry::lightmaps_sampler(5, sampler),
                ],
            )
        }
    }

    /// Creates the bind group for skinned meshes with no morph targets.
    pub fn skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "skinned_mesh_bind_group",
            &self.bind_group_layouts.skinned,
            &[
                entry::model(0, model.clone()),
                entry::metadata(9, metadata),
                entry::skinning(render_device, 1, current_skin),
            ],
        )
    }

    /// Creates the bind group for skinned meshes with no morph targets, with
    /// the infrastructure to compute motion vectors.
    ///
    /// `current_skin` is the buffer of joint matrices for this frame;
    /// `prev_skin` is the buffer for the previous frame. The latter is used for
    /// motion vector computation. If there is no such applicable buffer,
    /// `current_skin` and `prev_skin` will reference the same buffer.
    pub fn skinned_motion(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_skin: &Buffer,
        prev_skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(
            "skinned_motion_mesh_bind_group",
            &self.bind_group_layouts.skinned_motion,
            &[
                entry::model(0, model.clone()),
                entry::metadata(9, metadata),
                entry::skinning(render_device, 1, current_skin),
                entry::skinning(render_device, 6, prev_skin),
            ],
        )
    }

    /// Creates the bind group for meshes with no skins but morph targets.
    pub fn morphed(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_weights: &Buffer,
        targets: MorphTargetsResource,
        maybe_morph_descriptors: Option<&Buffer>,
    ) -> BindGroup {
        let mut entries: ArrayVec<BindGroupEntry, 5> = ArrayVec::new();

        entries.extend([
            entry::model(0, model.clone()),
            entry::metadata(9, metadata),
            entry::weights(render_device, 2, current_weights),
            entry::targets(3, targets),
        ]);

        if let Some(morph_descriptors) = maybe_morph_descriptors {
            entries.push(entry::morph_descriptors(8, morph_descriptors));
        }

        render_device.create_bind_group(
            "morphed_mesh_bind_group",
            &self.bind_group_layouts.morphed,
            &entries,
        )
    }

    /// Creates the bind group for meshes with no skins but morph targets, in
    /// addition to the infrastructure to compute motion vectors.
    ///
    /// `current_weights` is the buffer of morph weights for this frame;
    /// `prev_weights` is the buffer for the previous frame. The latter is used
    /// for motion vector computation. If there is no such applicable buffer,
    /// `current_weights` and `prev_weights` will reference the same buffer.
    pub fn morphed_motion(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_weights: &Buffer,
        prev_weights: &Buffer,
        targets: MorphTargetsResource,
        maybe_morph_descriptors: Option<&Buffer>,
    ) -> BindGroup {
        let mut entries: ArrayVec<BindGroupEntry, 6> = ArrayVec::new();

        entries.extend([
            entry::model(0, model.clone()),
            entry::metadata(9, metadata),
            entry::weights(render_device, 2, current_weights),
            entry::targets(3, targets),
            entry::weights(render_device, 7, prev_weights),
        ]);

        if let Some(morph_descriptors) = maybe_morph_descriptors {
            entries.push(entry::morph_descriptors(8, morph_descriptors));
        }

        render_device.create_bind_group(
            "morphed_motion_mesh_bind_group",
            &self.bind_group_layouts.morphed_motion,
            &entries,
        )
    }

    /// Creates the bind group for meshes with skins and morph targets.
    pub fn morphed_skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_skin: &Buffer,
        current_weights: &Buffer,
        targets: MorphTargetsResource,
        maybe_morph_descriptors: Option<&Buffer>,
    ) -> BindGroup {
        let mut entries: ArrayVec<BindGroupEntry, 6> = ArrayVec::new();

        entries.extend([
            entry::model(0, model.clone()),
            entry::metadata(9, metadata),
            entry::skinning(render_device, 1, current_skin),
            entry::weights(render_device, 2, current_weights),
            entry::targets(3, targets),
        ]);

        if let Some(morph_descriptors) = maybe_morph_descriptors {
            entries.push(entry::morph_descriptors(8, morph_descriptors));
        }

        render_device.create_bind_group(
            "morphed_skinned_mesh_bind_group",
            &self.bind_group_layouts.morphed_skinned,
            &entries,
        )
    }

    /// Creates the bind group for meshes with skins and morph targets, in
    /// addition to the infrastructure to compute motion vectors.
    ///
    /// See the documentation for [`MeshLayouts::skinned_motion`] and
    /// [`MeshLayouts::morphed_motion`] above for more information about the
    /// `current_skin`, `prev_skin`, `current_weights`, and `prev_weights`
    /// buffers.
    pub fn morphed_skinned_motion(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        metadata: &Buffer,
        current_skin: &Buffer,
        current_weights: &Buffer,
        targets: MorphTargetsResource,
        prev_skin: &Buffer,
        prev_weights: &Buffer,
        morph_descriptors: Option<&Buffer>,
    ) -> BindGroup {
        let mut entries: ArrayVec<BindGroupEntry, 8> = ArrayVec::new();

        entries.extend([
            entry::model(0, model.clone()),
            entry::metadata(9, metadata),
            entry::skinning(render_device, 1, current_skin),
            entry::weights(render_device, 2, current_weights),
            entry::targets(3, targets),
            entry::skinning(render_device, 6, prev_skin),
            entry::weights(render_device, 7, prev_weights),
        ]);

        if let Some(morph_descriptors) = morph_descriptors {
            entries.push(entry::morph_descriptors(8, morph_descriptors));
        }

        render_device.create_bind_group(
            "morphed_skinned_motion_mesh_bind_group",
            &self.bind_group_layouts.morphed_skinned_motion,
            &entries,
        )
    }
}
