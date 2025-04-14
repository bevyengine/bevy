use bevy_mesh::PrimitiveTopology;

use gltf::mesh::{Mesh, Mode, Primitive};

use crate::GltfError;

pub(crate) fn primitive_name(mesh: &Mesh<'_>, primitive: &Primitive) -> String {
    let mesh_name = mesh.name().unwrap_or("Mesh");
    if mesh.primitives().len() > 1 {
        format!("{}.{}", mesh_name, primitive.index())
    } else {
        mesh_name.to_string()
    }
}

/// Maps the `primitive_topology` from glTF to `wgpu`.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::result_large_err,
        reason = "`GltfError` is only barely past the threshold for large errors."
    )
)]
pub(crate) fn primitive_topology(mode: Mode) -> Result<PrimitiveTopology, GltfError> {
    match mode {
        Mode::Points => Ok(PrimitiveTopology::PointList),
        Mode::Lines => Ok(PrimitiveTopology::LineList),
        Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
        Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
        Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
        mode => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}
