//! Extra utilities for 3×3 matrices.

use glam::{Mat3A, Vec3, Vec3A};

/// Creates a 3×3 matrix that reflects points across the plane at the origin
/// with the given normal.
///
/// This is also known as a [Householder matrix]. It has the general form I -
/// 2NNᵀ, where N is the normal of the plane and I is the identity matrix.
///
/// If the plane across which points are to be reflected isn't at the origin,
/// you can create a translation matrix that translates the points to the
/// origin, then apply the matrix that this function returns on top of that, and
/// finally translate back to the original position.
///
/// See the `mirror` example for a demonstration of how you might use this
/// function.
///
/// [Householder matrix]: https://en.wikipedia.org/wiki/Householder_transformation
#[doc(alias = "householder")]
pub fn reflection_matrix(plane_normal: Vec3) -> Mat3A {
    // N times Nᵀ.
    let n_nt = Mat3A::from_cols(
        Vec3A::from(plane_normal) * plane_normal.x,
        Vec3A::from(plane_normal) * plane_normal.y,
        Vec3A::from(plane_normal) * plane_normal.z,
    );

    Mat3A::IDENTITY - n_nt * 2.0
}
