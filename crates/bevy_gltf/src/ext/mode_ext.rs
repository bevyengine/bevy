use bevy_render::mesh::PrimitiveTopology;

use crate::GltfError;

pub trait ModeExt {
    #[allow(clippy::result_large_err)]
    /// Get [`PrimitiveTopology`] of meshes [`Mode`](gltf::mesh::Mode)
    fn get_primitive_topology(&self) -> Result<PrimitiveTopology, GltfError>;
}

impl ModeExt for gltf::mesh::Mode {
    #[allow(clippy::result_large_err)]
    /// Maps the `primitive_topology` form glTF to `wgpu`.
    fn get_primitive_topology(&self) -> Result<PrimitiveTopology, GltfError> {
        match self {
            gltf::mesh::Mode::Points => Ok(PrimitiveTopology::PointList),
            gltf::mesh::Mode::Lines => Ok(PrimitiveTopology::LineList),
            gltf::mesh::Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
            gltf::mesh::Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
            gltf::mesh::Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
            &mode => Err(GltfError::UnsupportedPrimitive { mode }),
        }
    }
}
