use crate::shader::ShaderLayout;

impl ShaderLayout {
    pub fn from_spirv(_spirv_data: &[u32], _bevy_conventions: bool) -> ShaderLayout {
        panic!("reflecting shader layout from spirv data is not available");
    }
}
