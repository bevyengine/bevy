use std::marker::Copy;

#[derive(Copy, Clone)]
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
    let binary_result = compiler
        .compile_into_spirv(
            glsl_source,
            shader_kind,
            "shader.glsl",
            "main",
            Some(&options),
        )
        .unwrap();

    binary_result.as_binary().into()
}

pub enum ShaderSource {
    Spirv(Vec<u32>),
    Glsl(String),
}

pub struct Shader {
    pub source: ShaderSource,
    pub stage: ShaderStage,
    pub entry_point: String,
    pub macros: Option<Vec<String>>,
}

impl Shader {
    pub fn from_glsl(glsl: &str, stage: ShaderStage) -> Shader {
        Shader {
            source: ShaderSource::Glsl(glsl.to_string()),
            entry_point: "main".to_string(),
            macros: None,
            stage,
        }
    }

    pub fn get_spirv(&self) -> Vec<u32> {
        match self.source {
            ShaderSource::Spirv(ref bytes) => bytes.clone(),
            ShaderSource::Glsl(ref source) => glsl_to_spirv(&source, self.stage),
        }
    }

    pub fn create_shader_module(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(&self.get_spirv())
    }
}

pub struct ShaderStages {
    pub vertex: Shader,
    pub fragment: Option<Shader>,
}

impl ShaderStages {
    pub fn new(vertex_shader: Shader) -> Self {
        ShaderStages {
            vertex: vertex_shader,
            fragment: None,
        }
    }
}
