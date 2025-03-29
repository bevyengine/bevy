use bevy_math::Vec4;
use bevy_render::render_resource::ShaderType;

use super::mesh::Mesh2dTransforms;

#[derive(ShaderType, Clone, Copy)]
pub struct Mesh2dUniform {
    // Affine 4x3 matrix transposed to 3x4
    pub world_from_local: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub local_from_world_transpose_a: [Vec4; 2],
    pub local_from_world_transpose_b: f32,
    pub flags: u32,
    pub tag: u32,
}

impl Mesh2dUniform {
    pub fn from_components(mesh_transforms: &Mesh2dTransforms, tag: u32) -> Self {
        let (local_from_world_transpose_a, local_from_world_transpose_b) =
            mesh_transforms.world_from_local.inverse_transpose_3x3();
        Self {
            world_from_local: mesh_transforms.world_from_local.to_transpose(),
            local_from_world_transpose_a,
            local_from_world_transpose_b,
            flags: mesh_transforms.flags,
            tag,
        }
    }
}
