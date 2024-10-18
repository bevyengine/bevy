use std::io::Error;

use derive_more::derive::{Display, Error, From};

use gltf::mesh::Mode;

use bevy_asset::{AssetLoadError, ReadAssetBytesError};
use bevy_render::texture::TextureError;
use bevy_utils::tracing::error;

/// An error that occurs when loading a glTF file.
#[derive(Error, Display, Debug, From)]
pub enum GltfError {
    /// Unsupported primitive mode.
    #[display("unsupported primitive mode")]
    UnsupportedPrimitive {
        /// The primitive mode.
        mode: Mode,
    },
    /// Invalid glTF file.
    #[display("invalid glTF file: {_0}")]
    Gltf(gltf::Error),
    /// Binary blob is missing.
    #[display("binary blob is missing")]
    MissingBlob,
    /// Decoding the base64 mesh data failed.
    #[display("failed to decode base64 mesh data")]
    Base64Decode(base64::DecodeError),
    /// Unsupported buffer format.
    #[display("unsupported buffer format")]
    BufferFormatUnsupported,
    /// Invalid image mime type.
    #[display("invalid image mime type: {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    InvalidImageMimeType(String),
    /// Error when loading a texture. Might be due to a disabled image file format feature.
    #[display("You may need to add the feature for the file format: {_0}")]
    ImageError(TextureError),
    /// Failed to read bytes from an asset path.
    #[display("failed to read bytes from an asset path: {_0}")]
    ReadAssetBytesError(ReadAssetBytesError),
    /// Failed to load asset from an asset path.
    #[display("failed to load asset from an asset path: {_0}")]
    AssetLoadError(AssetLoadError),
    /// Missing sampler for an animation.
    #[display("Missing sampler for animation {_0}")]
    #[error(ignore)]
    #[from(ignore)]
    MissingAnimationSampler(usize),
    /// Failed to generate tangents.
    #[display("failed to generate tangents: {_0}")]
    GenerateTangentsError(bevy_render::mesh::GenerateTangentsError),
    /// Failed to generate morph targets.
    #[display("failed to generate morph targets: {_0}")]
    MorphTarget(bevy_render::mesh::morph::MorphBuildError),
    /// Circular children in Nodes
    #[display("GLTF model must be a tree, found cycle instead at node indices: {_0:?}")]
    #[error(ignore)]
    #[from(ignore)]
    CircularChildren(String),
    /// Failed to load a file.
    #[display("failed to load file: {_0}")]
    Io(Error),
}
