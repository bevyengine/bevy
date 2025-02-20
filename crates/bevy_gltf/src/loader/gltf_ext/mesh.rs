use bevy_render::mesh::PrimitiveTopology;

use gltf::mesh::{Mesh, Mode, Primitive};

use crate::GltfError;

pub trait MeshExt {
    fn primitive_name(&self, primitive: &Primitive) -> String;
}

pub trait ModeExt {
    #[expect(
        clippy::result_large_err,
        reason = "`GltfError` is only barely past the threshold for large errors."
    )]
    fn primitive_topology(self) -> Result<PrimitiveTopology, GltfError>;
}

impl MeshExt for Mesh<'_> {
    fn primitive_name(&self, primitive: &Primitive) -> String {
        let mesh_name = self.name().unwrap_or("Mesh");
        if self.primitives().len() > 1 {
            format!("{}.{}", mesh_name, primitive.index())
        } else {
            mesh_name.to_string()
        }
    }
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
