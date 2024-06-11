use super::triangle3d;
use crate::{
    mesh::{Indices, Mesh, MeshBuilder, Meshable},
    render_asset::RenderAssetUsages,
};
use bevy_math::primitives::{Tetrahedron, Triangle3d};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Tetrahedron`] shape.
pub struct TetrahedronMeshBuilder {
    tetrahedron: Tetrahedron,
}

impl MeshBuilder for TetrahedronMeshBuilder {
    fn build(&self) -> Mesh {
        let mut faces: Vec<_> = self.tetrahedron.faces().into();

        // If the tetrahedron has negative orientation, reverse all the triangles so that
        // they still face outward.
        if self.tetrahedron.signed_volume().is_sign_negative() {
            faces.iter_mut().for_each(Triangle3d::reverse);
        }

        let mut positions = vec![];
        let mut normals = vec![];
        let mut uvs = vec![];

        // Each face is meshed as a `Triangle3d`, and we just shove the data into the
        // vertex attributes sequentially.
        for face in faces {
            positions.extend(face.vertices);

            let face_normal = triangle3d::normal_vec(&face);
            normals.extend(vec![face_normal; 3]);

            let face_uvs = triangle3d::uv_coords(&face);
            uvs.extend(face_uvs);
        }

        // There are four faces and none of them share vertices.
        let indices = Indices::U32(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);

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

impl Meshable for Tetrahedron {
    type Output = TetrahedronMeshBuilder;

    fn mesh(&self) -> Self::Output {
        TetrahedronMeshBuilder { tetrahedron: *self }
    }
}

impl From<Tetrahedron> for Mesh {
    fn from(tetrahedron: Tetrahedron) -> Self {
        tetrahedron.mesh().build()
    }
}
