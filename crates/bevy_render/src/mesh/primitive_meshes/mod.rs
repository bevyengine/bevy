mod capsule;
mod circle;
mod cone;
mod cuboid;
mod cylinder;
mod rectangle;
mod regular_polygon;
mod sphere;
mod torus;
mod triangle;

pub(crate) mod builders {
    pub use super::capsule::CapsuleBuilder;
    pub use super::circle::CircleBuilder;
    pub use super::cone::ConeBuilder;
    pub use super::cylinder::CylinderBuilder;
    pub use super::rectangle::RectangleBuilder;
    pub use super::sphere::SphereBuilder;
    pub use super::torus::TorusBuilder;
}

use super::Mesh;

pub trait Meshable {
    type Output;

    fn mesh(&self) -> Self::Output;
}
