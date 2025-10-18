use crate::alpha::AlphaMode;
use crate::opaque::OpaqueRendererMethod;
use crate::render::MeshPipeline;
use crate::render::MeshPipelineKey;
use crate::render_phase::{
    DrawFunctionId, DrawFunctionLabel, InternedDrawFunctionLabel, InternedShaderLabel, ShaderLabel,
};
use crate::render_resource::{
    BindGroupLayoutDescriptor, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use crate::*;
use alloc::sync::Arc;
use bevy_asset::Handle;
use bevy_ecs::resource::Resource;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_platform::hash::FixedHasher;
use bevy_shader::Shader;
use core::any::{Any, TypeId};
use core::hash::Hash;
use core::hash::{BuildHasher, Hasher};
use smallvec::SmallVec;

pub const MATERIAL_BIND_GROUP_INDEX: usize = 3;

/// Render pipeline data for a given material.
#[derive(Resource, Clone)]
pub struct MaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErasedMaterialPipelineKey {
    pub mesh_key: MeshPipelineKey,
    pub material_key: ErasedMaterialKey,
    pub type_id: TypeId,
}

#[derive(Debug)]
pub struct ErasedMaterialKey {
    type_id: TypeId,
    hash: u64,
    value: Box<dyn Any + Send + Sync>,
    vtable: Arc<ErasedMaterialKeyVTable>,
}

#[derive(Debug)]
pub struct ErasedMaterialKeyVTable {
    clone_fn: fn(&dyn Any) -> Box<dyn Any + Send + Sync>,
    partial_eq_fn: fn(&dyn Any, &dyn Any) -> bool,
}

impl ErasedMaterialKey {
    pub fn new<T>(material_key: T) -> Self
    where
        T: Clone + Hash + PartialEq + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let hash = FixedHasher::hash_one(&FixedHasher, &material_key);

        fn clone<T: Clone + Send + Sync + 'static>(any: &dyn Any) -> Box<dyn Any + Send + Sync> {
            Box::new(any.downcast_ref::<T>().unwrap().clone())
        }
        fn partial_eq<T: PartialEq + 'static>(a: &dyn Any, b: &dyn Any) -> bool {
            a.downcast_ref::<T>().unwrap() == b.downcast_ref::<T>().unwrap()
        }

        Self {
            type_id,
            hash,
            value: Box::new(material_key),
            vtable: Arc::new(ErasedMaterialKeyVTable {
                clone_fn: clone::<T>,
                partial_eq_fn: partial_eq::<T>,
            }),
        }
    }

    pub fn to_key<T: Clone + 'static>(&self) -> T {
        debug_assert_eq!(self.type_id, TypeId::of::<T>());
        self.value.downcast_ref::<T>().unwrap().clone()
    }
}

impl PartialEq for ErasedMaterialKey {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
            && (self.vtable.partial_eq_fn)(self.value.as_ref(), other.value.as_ref())
    }
}

impl Eq for ErasedMaterialKey {}

impl Clone for ErasedMaterialKey {
    fn clone(&self) -> Self {
        Self {
            type_id: self.type_id,
            hash: self.hash,
            value: (self.vtable.clone_fn)(self.value.as_ref()),
            vtable: self.vtable.clone(),
        }
    }
}

impl Hash for ErasedMaterialKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.hash.hash(state);
    }
}

impl Default for ErasedMaterialKey {
    fn default() -> Self {
        Self::new(())
    }
}

/// Common material properties, calculated for a specific material instance.
#[derive(Default)]
pub struct MaterialProperties {
    /// Is this material should be rendered by the deferred renderer when.
    /// [`AlphaMode::Opaque`] or [`AlphaMode::Mask`]
    pub render_method: OpaqueRendererMethod,
    /// The [`AlphaMode`] of this material.
    pub alpha_mode: AlphaMode,
    /// The bits in the [`MeshPipelineKey`] for this material.
    ///
    /// These are precalculated so that we can just "or" them together.
    pub mesh_pipeline_key_bits: MeshPipelineKey,
    /// Add a bias to the view depth of the mesh which can be used to force a specific render order
    /// for meshes with equal depth, to avoid z-fighting.
    /// The bias is in depth-texture units so large values may be needed to overcome small depth differences.
    pub depth_bias: f32,
    /// Whether the material would like to read from a view transmission texture
    ///
    /// This allows taking color output from the opaque 3d pass as an input, (for screen-space transmission) but requires
    /// rendering to take place in a separate transmissive 3d pass.
    pub reads_view_transmission_texture: bool,
    pub render_phase_type: RenderPhaseType,
    pub material_layout: Option<BindGroupLayoutDescriptor>,
    /// Backing array is a size of 4 because the `StandardMaterial` needs 4 draw functions by default
    pub draw_functions: SmallVec<[(InternedDrawFunctionLabel, DrawFunctionId); 4]>,
    /// Backing array is a size of 3 because the `StandardMaterial` has 3 custom shaders (`frag`, `prepass_frag`, `deferred_frag`) which is the
    /// most common use case
    pub shaders: SmallVec<[(InternedShaderLabel, Handle<Shader>); 3]>,
    /// Whether this material *actually* uses bindless resources, taking the
    /// platform support (or lack thereof) of bindless resources into account.
    pub bindless: bool,
    pub specialize: Option<
        fn(
            &MaterialPipeline,
            &mut RenderPipelineDescriptor,
            &MeshVertexBufferLayoutRef,
            ErasedMaterialPipelineKey,
        ) -> Result<(), SpecializedMeshPipelineError>,
    >,
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

#[derive(Clone, Copy, Default)]
pub enum RenderPhaseType {
    #[default]
    Opaque,
    AlphaMask,
    Transmissive,
    Transparent,
}
