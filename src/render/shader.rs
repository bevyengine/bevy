#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl Into<shaderc::ShaderKind> for ShaderStage {
    fn into(self) -> shaderc::ShaderKind {
        match self {
            ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
            ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
            ShaderStage::Compute => shaderc::ShaderKind::Compute,
        }
    }
}

pub fn glsl_to_spirv(glsl_source: &str, stage: ShaderStage) -> Vec<u32> {
    let shader_kind: shaderc::ShaderKind = stage.into(); 
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let binary_result = compiler.compile_into_spirv(
        glsl_source, shader_kind,
        "shader.glsl", "main", Some(&options)).unwrap();
    
    binary_result.as_binary().into()
}