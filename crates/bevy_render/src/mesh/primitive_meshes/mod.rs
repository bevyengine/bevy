mod capsule;
mod circle;
mod cone;
mod conical_frustum;
mod cuboid;
mod cylinder;
mod rectangle;
mod regular_polygon;
mod sphere;
mod torus;
mod triangle;

pub(crate) mod builders {
    pub use super::capsule::CapsuleMesh;
    pub use super::circle::CircleMesh;
    pub use super::cone::ConeMesh;
    pub use super::conical_frustum::ConicalFrustumMesh;
    pub use super::cylinder::CylinderMesh;
    pub use super::rectangle::RectangleMesh;
    pub use super::sphere::SphereMesh;
    pub use super::torus::TorusMesh;
}

use super::Mesh;

pub trait Meshable {
    type Output;

    fn mesh(&self) -> Self::Output;
}
