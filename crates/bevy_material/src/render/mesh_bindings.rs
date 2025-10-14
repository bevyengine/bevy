use crate::render_resource::BindGroupLayoutDescriptor;

/// All possible [`BindGroupLayoutDescriptor`]s in bevy's default mesh shader (`mesh.wgsl`).
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

    /// Also includes the uniform and [`MorphAttributes`](`bevy_mesh::morph::MorphAttributes`) for morph targets.
    pub morphed: BindGroupLayoutDescriptor,

    /// Like [`MeshLayouts::morphed`], but includes a slot for the previous
    /// frame's morph weights, so that we can compute motion vectors.
    pub morphed_motion: BindGroupLayoutDescriptor,

    /// Also includes both uniforms for skinning and morph targets, also the
    /// morph target [`MorphAttributes`](`bevy_mesh::morph::MorphAttributes`) binding.
    pub morphed_skinned: BindGroupLayoutDescriptor,

    /// Like [`MeshLayouts::morphed_skinned`], but includes slots for the
    /// previous frame's joint matrices and morph weights, so that we can
    /// compute motion vectors.
    pub morphed_skinned_motion: BindGroupLayoutDescriptor,
}
