use super::ShaderLayout;
use bevy_asset::Handle;
use bevy_type_registry::TypeUuid;
use std::marker::Copy;

/// The stage of a shader
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[cfg(all(not(target_os = "ios"), not(target_arch = "wasm32")))]
impl Into<bevy_glsl_to_spirv::ShaderType> for ShaderStage {
    fn into(self) -> bevy_glsl_to_spirv::ShaderType {
        match self {
            ShaderStage::Vertex => bevy_glsl_to_spirv::ShaderType::Vertex,
            ShaderStage::Fragment => bevy_glsl_to_spirv::ShaderType::Fragment,
            ShaderStage::Compute => bevy_glsl_to_spirv::ShaderType::Compute,
        }
    }
}

#[cfg(all(not(target_os = "ios"), not(target_arch = "wasm32")))]
fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Vec<u32> {
    use std::io::Read;

    let mut output = bevy_glsl_to_spirv::compile(glsl_source, stage.into(), shader_defs).unwrap();
    let mut spv_bytes = Vec::new();
    output.read_to_end(&mut spv_bytes).unwrap();
    bytes_to_words(&spv_bytes)
}

#[cfg(target_os = "ios")]
impl Into<shaderc::ShaderKind> for ShaderStage {
    fn into(self) -> shaderc::ShaderKind {
        match self {
            ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
            ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
            ShaderStage::Compute => shaderc::ShaderKind::Compute,
        }
    }
}

#[cfg(target_os = "ios")]
fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Vec<u32> {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    if let Some(shader_defs) = shader_defs {
        for def in shader_defs.iter() {
            options.add_macro_definition(def, None);
        }
    }

    let binary_result = compiler
        .compile_into_spirv(
            glsl_source,
            stage.into(),
            "shader.glsl",
            "main",
            Some(&options),
        )
        .unwrap();

    binary_result.as_binary().to_vec()
}

fn bytes_to_words(bytes: &[u8]) -> Vec<u32> {
    let mut words = Vec::new();
    for bytes4 in bytes.chunks(4) {
        words.push(u32::from_le_bytes([
            bytes4[0], bytes4[1], bytes4[2], bytes4[3],
        ]));
    }

    words
}

/// The full "source" of a shader
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ShaderSource {
    Spirv(Vec<u32>),
    Glsl(String),
}

impl ShaderSource {
    pub fn spirv_from_bytes(bytes: &[u8]) -> ShaderSource {
        ShaderSource::Spirv(bytes_to_words(bytes))
    }
}

/// A shader, as defined by its [ShaderSource] and [ShaderStage]
#[derive(Clone, Debug, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    pub source: ShaderSource,
    pub stage: ShaderStage,
}

impl Shader {
    pub fn new(stage: ShaderStage, source: ShaderSource) -> Shader {
        Shader { stage, source }
    }

    pub fn from_glsl(stage: ShaderStage, glsl: &str) -> Shader {
        Shader {
            source: ShaderSource::Glsl(glsl.to_string()),
            stage,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_spirv(&self, macros: Option<&[String]>) -> Vec<u32> {
        match self.source {
            ShaderSource::Spirv(ref bytes) => bytes.clone(),
            ShaderSource::Glsl(ref source) => glsl_to_spirv(&source, self.stage, macros),
        }
    }

    #[allow(unused_variables)]
    pub fn get_spirv_shader(&self, macros: Option<&[String]>) -> Shader {
        Shader {
            #[cfg(not(target_arch = "wasm32"))]
            source: ShaderSource::Spirv(self.get_spirv(macros)),
            #[cfg(target_arch = "wasm32")]
            source: self.source.clone(),
            stage: self.stage,
        }
    }

    pub fn reflect_layout(&self, enforce_bevy_conventions: bool) -> Option<ShaderLayout> {
        if let ShaderSource::Spirv(ref spirv) = self.source {
            Some(ShaderLayout::from_spirv(
                spirv.as_slice(),
                enforce_bevy_conventions,
            ))
        } else {
            panic!("Cannot reflect layout of non-SpirV shader. Try compiling this shader to SpirV first using self.get_spirv_shader()");
        }
    }
}

/// All stages in a shader program
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ShaderStages {
    pub vertex: Handle<Shader>,
    pub fragment: Option<Handle<Shader>>,
}

pub struct ShaderStagesIterator<'a> {
    shader_stages: &'a ShaderStages,
    state: u32,
}

impl<'a> Iterator for ShaderStagesIterator<'a> {
    type Item = Handle<Shader>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = match self.state {
            0 => Some(self.shader_stages.vertex.clone_weak()),
            1 => self.shader_stages.fragment.as_ref().map(|h| h.clone_weak()),
            _ => None,
        };
        self.state += 1;
        ret
    }
}

impl ShaderStages {
    pub fn new(vertex_shader: Handle<Shader>) -> Self {
        ShaderStages {
            vertex: vertex_shader,
            fragment: None,
        }
    }

    pub fn iter(&self) -> ShaderStagesIterator {
        ShaderStagesIterator {
            shader_stages: &self,
            state: 0,
        }
    }
}
