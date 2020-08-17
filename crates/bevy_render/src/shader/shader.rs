use super::ShaderLayout;
use bevy_asset::Handle;
use std::marker::Copy;

#[cfg(feature = "naga-glsl")]
use super::preprocessor;
#[cfg(feature = "bevy-glsl-to-spirv")]
use bevy_glsl_to_spirv::compile;

/// The stage of a shader
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[cfg(feature = "naga-glsl")]
impl Into<naga::ShaderStage> for ShaderStage {
    fn into(self) -> naga::ShaderStage {
        match self {
            ShaderStage::Vertex => naga::ShaderStage::Vertex,
            ShaderStage::Fragment => naga::ShaderStage::Fragment,
            ShaderStage::Compute => naga::ShaderStage::Compute,
        }
    }
}

#[cfg(feature = "bevy-glsl-to-spirv")]
impl Into<bevy_glsl_to_spirv::ShaderType> for ShaderStage {
    fn into(self) -> bevy_glsl_to_spirv::ShaderType {
        match self {
            ShaderStage::Vertex => bevy_glsl_to_spirv::ShaderType::Vertex,
            ShaderStage::Fragment => bevy_glsl_to_spirv::ShaderType::Fragment,
            ShaderStage::Compute => bevy_glsl_to_spirv::ShaderType::Compute,
        }
    }
}

fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Vec<u32> {
    #[cfg(feature = "naga-glsl")]
    {
        let source = preprocessor::preprocess(
            glsl_source,
            |_include_path| Err(()), // bevy doesn't expose includes yet
            |found_definition| {
                shader_defs
                    .and_then(|defs| defs.find(|def| *def == found_definition))
                    .map(|def| if def.len() == 0 { None } else { Some(def) })
            },
        )
        .expect("unable to preprocess shader");

        // The `glsl_new` naga frontend is still a work-in-progress.
        let module =
            naga::front::glsl_new::parse_str(glsl_source, "main".to_string(), stage.into())
                .unwrap();
        println!("{:#?}", module);
        let mut writer =
            naga::back::spv::Writer::new(&module.header, naga::back::spv::WriterFlags::NONE);
        writer.write(&module)
    }
    #[cfg(feature = "bevy-glsl-to-spirv")]
    {
        use std::io::Read;

        let mut output = compile(glsl_source, stage.into(), shader_defs).unwrap();
        let mut spv_bytes = Vec::new();
        output.read_to_end(&mut spv_bytes).unwrap();
        bytes_to_words(&spv_bytes)
    }
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
#[derive(Clone, Debug)]
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
