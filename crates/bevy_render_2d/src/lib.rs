//! Provides functionality for rendering in 2d

pub mod material;
pub mod mesh_pipeline;

pub mod prelude {
    pub use super::material::{AlphaMode2d, Material2d, MeshMaterial2d};
}
