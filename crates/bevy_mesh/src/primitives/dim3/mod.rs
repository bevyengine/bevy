mod capsule;
mod cone;
mod conical_frustum;
mod cuboid;
mod cylinder;
mod plane;
mod sphere;
mod tetrahedron;
mod torus;
pub(crate) mod triangle3d;

pub use capsule::*;
pub use cone::*;
pub use conical_frustum::*;
pub use cuboid::*;
pub use cylinder::*;
pub use plane::*;
pub use sphere::*;
pub use tetrahedron::*;
pub use torus::*;
pub use triangle3d::*;

use bevy_math::{Dir3, Vec3};

fn calculate_tangents_around_axis(normals: &[[f32; 3]], tangent_axis: &Dir3) -> Vec<[f32; 4]> {
    normals
        .iter()
        .map(|normal| Vec3::from_array(*normal))
        .map(|normal| tangent_axis.cross(normal).extend(1.).to_array())
        .collect()
}
