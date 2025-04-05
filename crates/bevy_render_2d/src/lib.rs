//! Provides functionality for rendering in 2d

#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]

pub mod material;
pub mod mesh_pipeline;

pub mod prelude {
    pub use super::material::{AlphaMode2d, Material2d, MeshMaterial2d};
}
