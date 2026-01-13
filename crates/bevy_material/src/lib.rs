//! Provides a material abstraction for bevy
#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

use bevy_asset::Handle;
use bevy_shader::Shader;
use smallvec::SmallVec;

extern crate alloc;

use crate::{
    descriptor::BindGroupLayoutDescriptor,
    key::{ErasedMaterialKey, ErasedMeshPipelineKey},
    labels::{
        DrawFunctionId, DrawFunctionLabel, InternedDrawFunctionLabel, InternedShaderLabel,
        ShaderLabel,
    },
    specialize::{BaseSpecializeFn, PrepassSpecializeFn, UserSpecializeFn},
};

pub use crate::{alpha::AlphaMode, opaque::OpaqueRendererMethod, phase::RenderPhaseType};

mod alpha;
pub mod bind_group_layout_entries;
pub mod descriptor;
pub mod key;
pub mod labels;
mod opaque;
mod phase;
pub mod specialize;

/// The material prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::alpha::AlphaMode;
}

/// Common material properties, calculated for a specific material instance.
#[derive(Default)]
pub struct MaterialProperties {
    /// Is this material should be rendered by the deferred renderer when.
    /// [`AlphaMode::Opaque`] or [`AlphaMode::Mask`]
    pub render_method: OpaqueRendererMethod,
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
    /// The bits in the [`ErasedMeshPipelineKey`] for this material.
    ///
    /// These are precalculated so that we can just "or" them together in
    /// [`queue_material_meshes`](https://docs.rs/bevy/latest/bevy/pbr/fn.queue_material_meshes.html).
    pub mesh_pipeline_key_bits: ErasedMeshPipelineKey,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    pub depth_bias: f32,
    /// Whether the material would like to read from
    /// [`ViewTransmissionTexture`](https://docs.rs/bevy/latest/bevy/core_pipeline/core_3d/struct.ViewTransmissionTexture.html).
    ///
    /// This allows taking color output from the [`Opaque3d`](https://docs.rs/bevy/latest/bevy/core_pipeline/core_3d/struct.Opaque3d.html)
    /// pass as an input, (for screen-space transmission) but requires rendering to take place in a separate
    /// [`Transmissive3d`](https://docs.rs/bevy/latest/bevy/core_pipeline/core_3d/struct.Transmissive3d.html) pass.
    pub reads_view_transmission_texture: bool,
    pub render_phase_type: RenderPhaseType,
    pub material_layout: Option<BindGroupLayoutDescriptor>,
    /// Backing array is a size of 4 because the [`StandardMaterial`](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html)
    /// needs 4 draw functions by default
    pub draw_functions: SmallVec<[(InternedDrawFunctionLabel, DrawFunctionId); 4]>,
    /// Backing array is a size of 3 because the [`StandardMaterial`](https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html)
    /// has 3 custom shaders (`frag`, `prepass_frag`, `deferred_frag`) which is the
    /// most common use case
    pub shaders: SmallVec<[(InternedShaderLabel, Handle<Shader>); 3]>,
    /// Whether this material *actually* uses bindless resources, taking the
    /// platform support (or lack thereof) of bindless resources into account.
    pub bindless: bool,
    pub base_specialize: Option<BaseSpecializeFn>,
    pub prepass_specialize: Option<PrepassSpecializeFn>,
    pub user_specialize: Option<UserSpecializeFn>,
    /// The key for this material, typically a bitfield of flags that are used to modify
    /// the pipeline descriptor used for this material.
    pub material_key: ErasedMaterialKey,
    /// Whether shadows are enabled for this material
    pub shadows_enabled: bool,
    /// Whether prepass is enabled for this material
    pub prepass_enabled: bool,
}

impl MaterialProperties {
    pub fn get_shader(&self, label: impl ShaderLabel) -> Option<Handle<Shader>> {
        self.shaders
            .iter()
            .find(|(inner_label, _)| inner_label == &label.intern())
            .map(|(_, shader)| shader)
            .cloned()
    }

    pub fn add_shader(&mut self, label: impl ShaderLabel, shader: Handle<Shader>) {
        self.shaders.push((label.intern(), shader));
    }

    pub fn get_draw_function(&self, label: impl DrawFunctionLabel) -> Option<DrawFunctionId> {
        self.draw_functions
            .iter()
            .find(|(inner_label, _)| inner_label == &label.intern())
            .map(|(_, shader)| shader)
            .cloned()
    }

    pub fn add_draw_function(
        &mut self,
        label: impl DrawFunctionLabel,
        draw_function: DrawFunctionId,
    ) {
        self.draw_functions.push((label.intern(), draw_function));
    }
}
