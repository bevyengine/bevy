use bevy_render::mesh::PrimitiveTopology;
use gltf::mesh::Mode;

use crate::GltfError;

pub trait ModeExt {
    #[expect(
        clippy::result_large_err,
        reason = "`GltfError` is only barely past the threshold for large errors."
    )]
    fn primitive_topology(self) -> Result<PrimitiveTopology, GltfError>;
}

impl ModeExt for Mode {
    /// Maps the `primitive_topology` from glTF to `wgpu`.
    fn primitive_topology(self) -> Result<PrimitiveTopology, GltfError> {
        match self {
            Mode::Points => Ok(PrimitiveTopology::PointList),
            Mode::Lines => Ok(PrimitiveTopology::LineList),
            Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
            Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
            Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
            mode => Err(GltfError::UnsupportedPrimitive { mode }),
        }
    }
}
