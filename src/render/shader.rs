#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub fn load_glsl(code: &str, stage: ShaderStage) -> Vec<u32> {
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
        ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
    };

    wgpu::read_spirv(glsl_to_spirv::compile(&code, ty).unwrap()).unwrap()
}
