use std::marker::Copy;

#[derive(Hash, Eq, PartialEq, Copy, Clone)]
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

pub fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Vec<u32> {
    let shader_kind: shaderc::ShaderKind = stage.into();
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    if let Some(shader_defs) = shader_defs {
        for shader_def in shader_defs.iter() {
            options.add_macro_definition(shader_def.as_str(), None);
        }
    }
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

#[derive(Hash, Eq, PartialEq)]
pub enum ShaderSource {
    Spirv(Vec<u32>),
    Glsl(String),
}

#[derive(Hash, Eq, PartialEq)]
pub struct Shader {
    pub source: ShaderSource,
    pub stage: ShaderStage,
    pub entry_point: String,
    // TODO: add "precompile" flag?
}

impl Shader {
    pub fn from_glsl(glsl: &str, stage: ShaderStage) -> Shader {
        Shader {
            source: ShaderSource::Glsl(glsl.to_string()),
            entry_point: "main".to_string(),
            stage,
        }
    }

    pub fn get_spirv(&self, macros: Option<&[String]>) -> Vec<u32> {
        match self.source {
            ShaderSource::Spirv(ref bytes) => bytes.clone(),
            ShaderSource::Glsl(ref source) => {
                glsl_to_spirv(&source, self.stage, macros)
            }
        }
    }

    pub fn get_spirv_shader(&self, macros: Option<&[String]>) -> Shader {
        Shader {
            source: ShaderSource::Spirv(self.get_spirv(macros)),
            entry_point: self.entry_point.clone(),
            stage: self.stage,
        }
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
