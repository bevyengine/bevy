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

pub use capsule::CapsuleMesh;
pub use circle::CircleMesh;
pub use cone::ConeMesh;
pub use conical_frustum::ConicalFrustumMesh;
pub use cylinder::CylinderMesh;
pub use rectangle::RectangleMesh;
pub use sphere::SphereMesh;
pub use torus::TorusMesh;

use super::Mesh;

pub trait Meshable {
    type Output;

    fn mesh(&self) -> Self::Output;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Facing {
    X = 1,
    Y = 2,
    #[default]
    Z = 3,
    NegX = -1,
    NegY = -2,
    NegZ = -3,
}

impl Facing {
    /// Returns `1` if the facing direction is positive `X`, `Y`, or `Z`, and `-1` otherwise.
    pub const fn signum(&self) -> i8 {
        match self {
            Facing::X | Facing::Y | Facing::Z => 1,
            _ => -1,
        }
    }

    /// Returns the direction in as an array in the format `[x, y, z]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// assert_eq!(Facing::X.to_array(), [1.0, 0.0, 0.0]);
    /// ```
    pub const fn to_array(&self) -> [f32; 3] {
        match self {
            Facing::X => [1.0, 0.0, 0.0],
            Facing::Y => [0.0, 1.0, 0.0],
            Facing::Z => [0.0, 0.0, 1.0],
            Facing::NegX => [-1.0, 0.0, 0.0],
            Facing::NegY => [0.0, -1.0, 0.0],
            Facing::NegZ => [0.0, 0.0, -1.0],
        }
    }
}
