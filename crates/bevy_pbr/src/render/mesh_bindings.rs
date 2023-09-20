//! Bind group layout related definitions for the mesh pipeline.

use bevy_render::{
    mesh::morph::MAX_MORPH_WEIGHTS,
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
        BindingResource, Buffer, TextureView,
    },
    renderer::RenderDevice,
};

use crate::MeshPipelineKey;

const MORPH_WEIGHT_SIZE: usize = std::mem::size_of::<f32>();
pub const MORPH_BUFFER_SIZE: usize = MAX_MORPH_WEIGHTS * MORPH_WEIGHT_SIZE;

/// Individual layout entries.
mod layout_entry {
    use super::MORPH_BUFFER_SIZE;
    use crate::MeshUniform;
    use crate::{render::mesh::JOINT_BUFFER_SIZE, PreviousViewProjection};
    use bevy_render::{
        globals::GlobalsUniform,
        render_resource::{
            BindGroupLayoutEntry, BindingType, BufferBindingType, BufferSize, GpuArrayBuffer,
            ShaderStages, ShaderType, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        view::ViewUniform,
    };

    fn buffer_with_offset(
        binding: u32,
        size: u64,
        visibility: ShaderStages,
        has_dynamic_offset: bool,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            count: None,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset,
                min_binding_size: BufferSize::new(size),
            },
        }
    }
    fn buffer(binding: u32, size: u64, visibility: ShaderStages) -> BindGroupLayoutEntry {
        buffer_with_offset(binding, size, visibility, true)
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

    // ---------- Prepass bind group layouts ----------

    pub(super) fn view(binding: u32) -> BindGroupLayoutEntry {
        let size = ViewUniform::min_size().get();
        buffer(binding, size, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
    }
    pub(super) fn globals(binding: u32) -> BindGroupLayoutEntry {
        let size = GlobalsUniform::min_size().get();
        buffer_with_offset(binding, size, ShaderStages::VERTEX, false)
    }
    pub(super) fn previous_view_projection(binding: u32) -> BindGroupLayoutEntry {
        let size = PreviousViewProjection::min_size().get();
        buffer(binding, size, ShaderStages::VERTEX | ShaderStages::FRAGMENT)
    }
}
/// Individual [`BindGroupEntry`](bevy_render::render_resource::BindGroupEntry)
/// for bind groups.
mod entry {
    use super::MORPH_BUFFER_SIZE;
    use crate::render::mesh::JOINT_BUFFER_SIZE;
    use bevy_render::render_resource::{
        BindGroupEntry, BindingResource, Buffer, BufferBinding, BufferSize, TextureView,
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
    pub(super) fn resource(binding: u32, resource: BindingResource) -> BindGroupEntry {
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
}

/// All possible [`BindGroupLayout`]s in bevy's default mesh shader (`mesh.wgsl`).
#[derive(Clone)]
pub struct MeshLayouts {
    pub mv: MvMeshLayouts,
    pub no_mv: MvMeshLayouts,
}

#[derive(Clone)]
pub struct MvMeshLayouts {
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
            mv: MvMeshLayouts::new_mv(render_device),
            no_mv: MvMeshLayouts::new(render_device),
        }
    }
    // ---------- BindGroup methods ----------

    pub fn model_only(&self, render_device: &RenderDevice, model: &BindingResource) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::resource(0, model.clone())],
            layout: &self.no_mv.model_only,
            label: Some("model_only_mesh_bind_group"),
        })
    }
    pub fn skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::resource(0, model.clone()), entry::skinning(1, skin)],
            layout: &self.no_mv.skinned,
            label: Some("skinned_mesh_bind_group"),
        })
    }
    pub fn morphed(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, model.clone()),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
            layout: &self.no_mv.morphed,
            label: Some("morphed_mesh_bind_group"),
        })
    }
    pub fn morphed_skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, model.clone()),
                entry::skinning(1, skin),
                entry::weights(2, weights),
                entry::targets(3, targets),
            ],
            layout: &self.no_mv.morphed_skinned,
            label: Some("morphed_skinned_mesh_bind_group"),
        })
    }
    // ---------- Motion Vectors BindGroup methods ----------

    pub fn mv_model_only(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[entry::resource(0, model.clone())],
            layout: &self.mv.model_only,
            label: Some("mv_model_only_mesh_bind_group"),
        })
    }
    pub fn mv_skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        old_skin: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, model.clone()),
                entry::skinning(1, skin),
                entry::skinning(4, old_skin),
            ],
            layout: &self.mv.skinned,
            label: Some("mv_skinned_mesh_bind_group"),
        })
    }
    pub fn mv_morphed(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        weights: &Buffer,
        targets: &TextureView,
        old_weights: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, model.clone()),
                entry::weights(2, weights),
                entry::targets(3, targets),
                entry::weights(5, old_weights),
            ],
            layout: &self.mv.morphed,
            label: Some("mv_morphed_mesh_bind_group"),
        })
    }
    pub fn mv_morphed_skinned(
        &self,
        render_device: &RenderDevice,
        model: &BindingResource,
        skin: &Buffer,
        weights: &Buffer,
        targets: &TextureView,
        old_skin: &Buffer,
        old_weights: &Buffer,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, model.clone()),
                entry::skinning(1, skin),
                entry::weights(2, weights),
                entry::targets(3, targets),
                entry::skinning(4, old_skin),
                entry::weights(5, old_weights),
            ],
            layout: &self.mv.morphed_skinned,
            label: Some("mv_morphed_skinned_mesh_bind_group"),
        })
    }
}

impl MvMeshLayouts {
    /// Prepare the layouts used by the default bevy [`Mesh`].
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new(render_device: &RenderDevice) -> Self {
        MvMeshLayouts {
            model_only: Self::model_only_layout(render_device),
            skinned: Self::skinned_layout(render_device),
            morphed: Self::morphed_layout(render_device),
            morphed_skinned: Self::morphed_skinned_layout(render_device),
        }
    }

    /// Prepare the layouts used by the default bevy [`Mesh`], with motion vector bindings.
    ///
    /// [`Mesh`]: bevy_render::prelude::Mesh
    pub fn new_mv(render_device: &RenderDevice) -> Self {
        MvMeshLayouts {
            model_only: Self::mv_model_only_layout(render_device),
            skinned: Self::mv_skinned_layout(render_device),
            morphed: Self::mv_morphed_layout(render_device),
            morphed_skinned: Self::mv_morphed_skinned_layout(render_device),
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn model_only_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(render_device, 0)],
            label: Some("mesh_layout"),
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
    // ---------- Motion Vectors create individual BindGroupLayouts ----------

    fn mv_model_only_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::model(render_device, 0)],
            label: Some("mv_mesh_layout"),
        })
    }
    fn mv_skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::skinning(1),
                layout_entry::skinning(4),
            ],
            label: Some("mv_skinned_mesh_layout"),
        })
    }
    fn mv_morphed_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::weights(2),
                layout_entry::targets(3),
                layout_entry::weights(5),
            ],
            label: Some("mv_morphed_mesh_layout"),
        })
    }
    fn mv_morphed_skinned_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::model(render_device, 0),
                layout_entry::skinning(1),
                layout_entry::weights(2),
                layout_entry::targets(3),
                layout_entry::skinning(4),
                layout_entry::weights(5),
            ],
            label: Some("mv_morphed_skinned_mesh_layout"),
        })
    }
}

/// All possible [`BindGroupLayout`]s in bevy's prepass shader (`prepass.wgsl`).
#[derive(Clone)]
pub struct MotionVectorsPrepassLayouts {
    pub without: BindGroupLayout,
    pub with: BindGroupLayout,
}
impl MotionVectorsPrepassLayouts {
    /// Prepare the layouts used by the prepass shader.
    pub fn new(render_device: &RenderDevice) -> Self {
        MotionVectorsPrepassLayouts {
            without: Self::without_layout(render_device),
            with: Self::with_layout(render_device),
        }
    }

    // ---------- create individual BindGroupLayouts ----------

    fn without_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[layout_entry::view(0), layout_entry::globals(1)],
            label: Some("prepass_no_motion_vectors_layout"),
        })
    }
    fn with_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                layout_entry::view(0),
                layout_entry::globals(1),
                layout_entry::previous_view_projection(2),
            ],
            label: Some("prepass_model_only_layout"),
        })
    }

    // ---------- BindGroup methods ----------

    pub fn without(
        &self,
        render_device: &RenderDevice,
        view: &BindingResource,
        globals: &BindingResource,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, view.clone()),
                entry::resource(1, globals.clone()),
            ],
            layout: &self.without,
            label: Some("prepass_mvectors_without_bind_group"),
        })
    }
    pub fn with(
        &self,
        render_device: &RenderDevice,
        view: &BindingResource,
        globals: &BindingResource,
        previous_view_proj: &BindingResource,
    ) -> BindGroup {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                entry::resource(0, view.clone()),
                entry::resource(1, globals.clone()),
                entry::resource(2, previous_view_proj.clone()),
            ],
            layout: &self.with,
            label: Some("prepass_mvectors_with_bind_group"),
        })
    }

    pub fn for_shader_defs(&self, key: &MeshPipelineKey) -> &BindGroupLayout {
        if key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            &self.with
        } else {
            &self.without
        }
    }
}
