use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use crate::mesh::Indices;
use bevy_math::primitives::Rectangle;
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug, Default)]
pub struct RectangleMesh {
    pub rectangle: Rectangle,
    pub facing: Facing,
}

impl MeshFacingExtension for RectangleMesh {
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl RectangleMesh {
    /// Creates a new [`RectangleMesh`] from a given radius and vertex count.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            rectangle: Rectangle::new(width, height),
            facing: Facing::Z,
        }
    }

    fn build(&self) -> Mesh {
        let [hw, hh] = [self.rectangle.half_width, self.rectangle.half_height];
        let positions = match self.facing {
            Facing::Z | Facing::NegZ => vec![
                [hw, hh, 0.0],
                [-hw, hh, 0.0],
                [-hw, -hh, 0.0],
                [hw, -hh, 0.0],
            ],
            Facing::Y | Facing::NegY => vec![
                [hw, 0.0, -hh],
                [-hw, 0.0, -hh],
                [-hw, 0.0, hh],
                [hw, 0.0, hh],
            ],
            Facing::X | Facing::NegX => vec![
                [0.0, hh, -hw],
                [0.0, hh, hw],
                [0.0, -hh, hw],
                [0.0, -hh, -hw],
            ],
        };

        let normals = vec![self.facing.to_array(); 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

        // Flip indices if facing -X, -Y, or -Z
        let indices = if self.facing.signum() > 0 {
            Indices::U32(vec![0, 1, 2, 0, 2, 3])
        } else {
            Indices::U32(vec![0, 2, 1, 0, 3, 2])
        };

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Rectangle {
    type Output = RectangleMesh;

    fn mesh(&self) -> Self::Output {
        RectangleMesh {
            rectangle: *self,
            ..Default::default()
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
