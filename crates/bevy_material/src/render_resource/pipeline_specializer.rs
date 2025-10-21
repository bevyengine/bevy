use bevy_mesh::MissingVertexAttributeError;
use core::fmt::Debug;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum SpecializedMeshPipelineError {
    #[error(transparent)]
    MissingVertexAttribute(#[from] MissingVertexAttributeError),
}
