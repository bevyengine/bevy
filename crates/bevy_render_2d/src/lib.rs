//! Infrastructure for rendering in 2d

pub mod material;
pub mod mesh;
#[cfg(feature = "wireframe")]
pub mod wireframe;

#[cfg(feature = "wireframe")]
pub use wireframe::Wireframe2dPlugin;
pub use {material::Material2dPlugin, mesh::Mesh2dRenderPlugin};

#[doc(hidden)]
pub mod prelude {
    pub use super::material::{rendering::AlphaMode2d, Material2d, MeshMaterial2d};
}
