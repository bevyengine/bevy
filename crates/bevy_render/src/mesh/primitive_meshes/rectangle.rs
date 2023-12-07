use super::{Mesh, Meshable};
use crate::mesh::Indices;
use bevy_math::primitives::Rectangle;
use wgpu::PrimitiveTopology;

#[derive(Debug)]
pub struct RectangleMesh {
    pub rectangle: Rectangle,
    pub flipped: bool,
}

impl RectangleMesh {
    fn build(&self) -> Mesh {
        let (u_left, u_right) = if self.flipped { (1.0, 0.0) } else { (0.0, 1.0) };
        let [hw, hh] = [self.rectangle.half_width, self.rectangle.half_height];
        let vertices = [
            ([-hw, -hh, 0.0], [0.0, 0.0, 1.0], [u_left, 1.0]),
            ([-hw, hh, 0.0], [0.0, 0.0, 1.0], [u_left, 0.0]),
            ([hw, hh, 0.0], [0.0, 0.0, 1.0], [u_right, 0.0]),
            ([hw, -hh, 0.0], [0.0, 0.0, 1.0], [u_right, 1.0]),
        ];

        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }

    /// Flips the UVs of the rectangle mesh.
    pub fn flipped(mut self) -> Self {
        self.flipped = !self.flipped;
        self
    }
}

impl Meshable for Rectangle {
    type Output = RectangleMesh;

    fn mesh(&self) -> Self::Output {
        RectangleMesh {
            rectangle: *self,
            flipped: false,
        }
    }
}

impl From<Rectangle> for Mesh {
    fn from(rectangle: Rectangle) -> Self {
        rectangle.mesh().build()
    }
}

impl From<RectangleMesh> for Mesh {
    fn from(rectangle: RectangleMesh) -> Self {
        rectangle.build()
    }
}
