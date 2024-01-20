use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use crate::{mesh::Indices, render_asset::RenderAssetPersistencePolicy};
use bevy_math::primitives::Rectangle;
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Rectangle`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct RectangleMeshBuilder {
    /// The [`Rectangle`] shape.
    pub rectangle: Rectangle,
    /// The XYZ direction that the mesh is facing.
    /// The default is [`Facing::Z`].
    pub facing: Facing,
}

impl MeshFacingExtension for RectangleMeshBuilder {
    #[inline]
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl RectangleMeshBuilder {
    /// Creates a new [`RectangleMesh`] from a given `width` and `height`.
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            rectangle: Rectangle::new(width, height),
            facing: Facing::Z,
        }
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
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

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetPersistencePolicy::Keep,
        )
        .with_indices(Some(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Rectangle {
    type Output = RectangleMeshBuilder;

    fn mesh(&self) -> Self::Output {
        RectangleMeshBuilder {
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

impl From<RectangleMeshBuilder> for Mesh {
    fn from(rectangle: RectangleMeshBuilder) -> Self {
        rectangle.build()
    }
}
