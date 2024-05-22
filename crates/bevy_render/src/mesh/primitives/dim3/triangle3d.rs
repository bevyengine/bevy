use bevy_math::{primitives::Triangle3d, Vec3};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};

/// A builder used for creating a [`Mesh`] with a [`Triangle3d`] shape.
pub struct Triangle3dMeshBuilder {
    triangle: Triangle3d,
}

impl MeshBuilder for Triangle3dMeshBuilder {
    fn build(&self) -> Mesh {
        let positions: Vec<_> = self.triangle.vertices.into();
        let uvs: Vec<_> = uv_coords(&self.triangle).into();

        // Every vertex has the normal of the face of the triangle (or zero if the triangle is degenerate).
        let normal: Vec3 = normal_vec(&self.triangle);
        let normals = vec![normal; 3];

        let indices = Indices::U32(vec![0, 1, 2]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Triangle3d {
    type Output = Triangle3dMeshBuilder;

    fn mesh(&self) -> Self::Output {
        Triangle3dMeshBuilder { triangle: *self }
    }
}

/// The normal of a [`Triangle3d`] with zeroing so that a [`Vec3`] is always obtained for meshing.
#[inline]
pub(crate) fn normal_vec(triangle: &Triangle3d) -> Vec3 {
    triangle.normal().map_or(Vec3::ZERO, |n| n.into())
}

/// Unskewed uv-coordinates for a [`Triangle3d`].
#[inline]
pub(crate) fn uv_coords(triangle: &Triangle3d) -> [[f32; 2]; 3] {
    let [a, b, c] = triangle.vertices;

    let main_length = a.distance(b);
    let Some(x) = (b - a).try_normalize() else {
        return [[0., 0.], [1., 0.], [0., 1.]];
    };
    let y = c - a;

    // `x` corresponds to one of the axes in uv-coordinates;
    // to uv-map the triangle without skewing, we use the orthogonalization
    // of `y` with respect to `x` as the second direction and construct a rectangle that
    // contains `triangle`.
    let y_proj = y.project_onto_normalized(x);

    // `offset` represents the x-coordinate of the point `c`; note that x has been shrunk by a
    // factor of `main_length`, so `offset` follows it.
    let offset = y_proj.dot(x) / main_length;

    // Obtuse triangle leaning to the left => x direction extends to the left, shifting a from 0.
    if offset < 0. {
        let total_length = 1. - offset;
        let a_uv = [offset.abs() / total_length, 0.];
        let b_uv = [1., 0.];
        let c_uv = [0., 1.];

        [a_uv, b_uv, c_uv]
    }
    // Obtuse triangle leaning to the right => x direction extends to the right, shifting b from 1.
    else if offset > 1. {
        let a_uv = [0., 0.];
        let b_uv = [1. / offset, 0.];
        let c_uv = [1., 1.];

        [a_uv, b_uv, c_uv]
    }
    // Acute triangle => no extending necessary; a remains at 0 and b remains at 1.
    else {
        let a_uv = [0., 0.];
        let b_uv = [1., 0.];
        let c_uv = [offset, 1.];

        [a_uv, b_uv, c_uv]
    }
}

impl From<Triangle3d> for Mesh {
    fn from(triangle: Triangle3d) -> Self {
        triangle.mesh().build()
    }
}

#[cfg(test)]
mod tests {
    use super::uv_coords;
    use bevy_math::primitives::Triangle3d;

    #[test]
    fn uv_test() {
        use bevy_math::vec3;
        let mut triangle = Triangle3d::new(vec3(0., 0., 0.), vec3(2., 0., 0.), vec3(-1., 1., 0.));

        let [a_uv, b_uv, c_uv] = uv_coords(&triangle);
        assert_eq!(a_uv, [1. / 3., 0.]);
        assert_eq!(b_uv, [1., 0.]);
        assert_eq!(c_uv, [0., 1.]);

        triangle.vertices[2] = vec3(3., 1., 0.);
        let [a_uv, b_uv, c_uv] = uv_coords(&triangle);
        assert_eq!(a_uv, [0., 0.]);
        assert_eq!(b_uv, [2. / 3., 0.]);
        assert_eq!(c_uv, [1., 1.]);

        triangle.vertices[2] = vec3(2., 1., 0.);
        let [a_uv, b_uv, c_uv] = uv_coords(&triangle);
        assert_eq!(a_uv, [0., 0.]);
        assert_eq!(b_uv, [1., 0.]);
        assert_eq!(c_uv, [1., 1.]);
    }
}
