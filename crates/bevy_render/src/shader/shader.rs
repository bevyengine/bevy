use super::ShaderLayout;
use bevy_asset::Handle;
use glsl_to_spirv::compile;
use std::{io::Read, marker::Copy};
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl Into<glsl_to_spirv::ShaderType> for ShaderStage {
    fn into(self) -> glsl_to_spirv::ShaderType {
        match self {
            ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
            ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
            ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
        }
    }
}

pub fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Vec<u32> {
    let mut output = compile(glsl_source, stage.into(), shader_defs).unwrap();
    let mut spv_bytes = Vec::new();
    output.read_to_end(&mut spv_bytes).unwrap();

    let mut spv_words = Vec::new();
    for bytes4 in spv_bytes.chunks(4) {
        spv_words.push(u32::from_le_bytes([
            bytes4[0], bytes4[1], bytes4[2], bytes4[3],
        ]));
    }
    spv_words
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum ShaderSource {
    Spirv(Vec<u32>),
    Glsl(String),
}

#[derive(Clone, Debug)]
pub struct Shader {
    pub source: ShaderSource,
    pub stage: ShaderStage,
    // TODO: add "precompile" flag?
}

impl Shader {
    pub fn from_glsl(stage: ShaderStage, glsl: &str) -> Shader {
        Shader {
            source: ShaderSource::Glsl(glsl.to_string()),
            stage,
        }
    }

    pub fn get_spirv(&self, macros: Option<&[String]>) -> Vec<u32> {
        match self.source {
            ShaderSource::Spirv(ref bytes) => bytes.clone(),
            ShaderSource::Glsl(ref source) => glsl_to_spirv(&source, self.stage, macros),
        }
    }

    pub fn get_spirv_shader(&self, macros: Option<&[String]>) -> Shader {
        Shader {
            source: ShaderSource::Spirv(self.get_spirv(macros)),
            stage: self.stage,
        }
    }

    pub fn reflect_layout(&self) -> Option<ShaderLayout> {
        if let ShaderSource::Spirv(ref spirv) = self.source {
            Some(ShaderLayout::from_spirv(spirv.as_slice()))
        } else {
            panic!("Cannot reflect layout of non-SpirV shader. Try compiling this shader to SpirV first using self.get_spirv_shader()");
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShaderStages {
    pub vertex: Handle<Shader>,
    pub fragment: Option<Handle<Shader>>,
}

impl ShaderStages {
    pub fn new(vertex_shader: Handle<Shader>) -> Self {
        ShaderStages {
            vertex: vertex_shader,
            fragment: None,
        }
    }
}
