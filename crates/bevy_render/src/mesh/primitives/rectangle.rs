use super::{Mesh, Meshable};
use crate::{mesh::Indices, render_asset::RenderAssetPersistencePolicy};
use bevy_math::primitives::Rectangle;
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Rectangle`] shape.
#[derive(Clone, Copy, Debug, Default)]
pub struct RectangleMeshBuilder {
    /// The [`Rectangle`] shape.
    pub rectangle: Rectangle,
}

impl RectangleMeshBuilder {
    /// Creates a new [`RectangleMeshBuilder`] from a given `width` and `height`.
    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            rectangle: Rectangle::new(width, height),
        }
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let [hw, hh] = [self.rectangle.half_size.x, self.rectangle.half_size.y];
        let positions = vec![
            [hw, hh, 0.0],
            [-hw, hh, 0.0],
            [-hw, -hh, 0.0],
            [hw, -hh, 0.0],
        ];
        let normals = vec![[0.0, 0.0, 1.0]; 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

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
        RectangleMeshBuilder { rectangle: *self }
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
