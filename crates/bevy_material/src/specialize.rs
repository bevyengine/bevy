use bevy_ecs::world::World;
use bevy_mesh::{MeshVertexBufferLayoutRef, MissingVertexAttributeError};
use bevy_platform::sync::Arc;
use core::any::Any;
use thiserror::Error;
use wgpu_types::TextureFormat;

use crate::{
    descriptor::{CachedRenderPipelineId, RenderPipelineDescriptor},
    key::ErasedMaterialPipelineKey,
    MaterialProperties,
};

/// A type erased function pointer for specializing a material pipeline. The implementation is
/// expected to:
/// - Look up the appropriate specializer from the world
/// - Downcast the erased key to the concrete key type
/// - Call `SpecializedMeshPipelines::specialize` with the specializer and return the resulting pipeline id
pub type BaseSpecializeFn = fn(
    &mut World,
    ErasedMaterialPipelineKey,
    &MeshVertexBufferLayoutRef,
    &Arc<MaterialProperties>,
) -> Result<CachedRenderPipelineId, SpecializedMeshPipelineError>;

/// A type erased function pointer for specializing a material prepass pipeline. The implementation is
/// expected to:
/// - Look up the appropriate specializer from the world
/// - Downcast the erased key to the concrete key type
/// - Call `SpecializedMeshPipelines::specialize` with the specializer and return the resulting pipeline id
pub type PrepassSpecializeFn = fn(
    &mut World,
    &ErasedMaterialPipelineKey,
    &MeshVertexBufferLayoutRef,
    &Arc<MaterialProperties>,
) -> Result<CachedRenderPipelineId, SpecializedMeshPipelineError>;

/// A type erased function pointer for specializing a material prepass pipeline. The implementation is
/// expected to:
/// - Look up the appropriate specializer from the world
/// - Downcast the erased key to the concrete key type
/// - Call `SpecializedMeshPipelines::specialize` with the specializer and return the resulting pipeline id
pub type UserSpecializeFn = fn(
    &dyn Any,
    &mut RenderPipelineDescriptor,
    &MeshVertexBufferLayoutRef,
    ErasedMaterialPipelineKey,
) -> Result<(), SpecializedMeshPipelineError>;

#[derive(Error, Debug)]
pub enum SpecializedMeshPipelineError {
    #[error(transparent)]
    MissingVertexAttribute(#[from] MissingVertexAttributeError),
    #[error(
        "Material requires stencil operations, but the depth-stencil format '{format:?}' lacks a stencil aspect. \
        Please configure Camera3d with a stencil-compatible depth format such as Depth24PlusStencil8 or Depth32FloatStencil8."
    )]
    IncompatibleDepthStencilFormat { format: TextureFormat },
}
