mod circle;
mod cuboid;
mod rectangle;
mod regular_polygon;
mod sphere;
mod triangle;

pub(crate) mod builders {
    pub use super::circle::CircleBuilder;
    pub use super::rectangle::RectangleBuilder;
    pub use super::sphere::SphereBuilder;
}

use super::Mesh;

pub trait Meshable {
    type Output;

    fn mesh(&self) -> Self::Output;
}
