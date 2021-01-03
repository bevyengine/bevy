use crate::{
    pipeline::{PipelineCompiler, PipelineDescriptor},
    renderer::RenderResourceContext,
};

use super::ShaderLayout;
use bevy_app::{EventReader, Events};
use bevy_asset::{AssetEvent, AssetLoader, Assets, Handle, LoadContext, LoadedAsset};
use bevy_ecs::{Local, Res, ResMut};
use bevy_reflect::TypeUuid;
use bevy_utils::{tracing::error, BoxedFuture};
use std::marker::Copy;
use thiserror::Error;

/// The stage of a shader
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

/// An error that occurs during shader handling.
#[derive(Error, Debug)]
pub enum ShaderError {
    /// Shader compilation error.
    #[error("Shader compilation error: {0}")]
    Compilation(String),

    #[cfg(any(target_os = "ios", all(target_arch = "aarch64", target_os = "macos")))]
    /// shaderc error.
    #[error("shaderc error")]
    ShaderC(#[from] shaderc::Error),

    #[cfg(any(target_os = "ios", all(target_arch = "aarch64", target_os = "macos")))]
    #[error("Error initializing shaderc Compiler")]
    ErrorInitializingShadercCompiler,

    #[cfg(any(target_os = "ios", all(target_arch = "aarch64", target_os = "macos")))]
    #[error("Error initializing shaderc CompileOptions")]
    ErrorInitializingShadercCompileOptions,
}

#[cfg(all(
    not(target_os = "ios"),
    not(target_arch = "wasm32"),
    not(all(target_arch = "aarch64", target_os = "macos"))
))]
impl Into<bevy_glsl_to_spirv::ShaderType> for ShaderStage {
    fn into(self) -> bevy_glsl_to_spirv::ShaderType {
        match self {
            ShaderStage::Vertex => bevy_glsl_to_spirv::ShaderType::Vertex,
            ShaderStage::Fragment => bevy_glsl_to_spirv::ShaderType::Fragment,
            ShaderStage::Compute => bevy_glsl_to_spirv::ShaderType::Compute,
        }
    }
}

#[cfg(all(
    not(target_os = "ios"),
    not(target_arch = "wasm32"),
    not(all(target_arch = "aarch64", target_os = "macos"))
))]
pub fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Result<Vec<u32>, ShaderError> {
    bevy_glsl_to_spirv::compile(glsl_source, stage.into(), shader_defs)
        .map_err(ShaderError::Compilation)
}

#[cfg(any(target_os = "ios", all(target_arch = "aarch64", target_os = "macos")))]
impl Into<shaderc::ShaderKind> for ShaderStage {
    fn into(self) -> shaderc::ShaderKind {
        match self {
            ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
            ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
            ShaderStage::Compute => shaderc::ShaderKind::Compute,
        }
    }
}

#[cfg(any(target_os = "ios", all(target_arch = "aarch64", target_os = "macos")))]
pub fn glsl_to_spirv(
    glsl_source: &str,
    stage: ShaderStage,
    shader_defs: Option<&[String]>,
) -> Result<Vec<u32>, ShaderError> {
    let mut compiler =
        shaderc::Compiler::new().ok_or(ShaderError::ErrorInitializingShadercCompiler)?;
    let mut options = shaderc::CompileOptions::new()
        .ok_or(ShaderError::ErrorInitializingShadercCompileOptions)?;
    if let Some(shader_defs) = shader_defs {
        for def in shader_defs.iter() {
            options.add_macro_definition(def, None);
        }
    }

    let binary_result = compiler.compile_into_spirv(
        glsl_source,
        stage.into(),
        "shader.glsl",
        "main",
        Some(&options),
    )?;

    Ok(binary_result.as_binary().to_vec())
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_spirv(spirv: &[u8]) -> Result<Shader, ShaderError> {
        use spirv_reflect::{types::ReflectShaderStageFlags, ShaderModule};

        let module = ShaderModule::load_u8_data(spirv)
            .map_err(|msg| ShaderError::Compilation(msg.to_string()))?;
        let stage = match module.get_shader_stage() {
            ReflectShaderStageFlags::VERTEX => ShaderStage::Vertex,
            ReflectShaderStageFlags::FRAGMENT => ShaderStage::Fragment,
            other => panic!("cannot load {:?} shader", other),
        };

        Ok(Shader {
            source: ShaderSource::spirv_from_bytes(spirv),
            stage,
        })
    }

    pub fn from_glsl(stage: ShaderStage, glsl: &str) -> Shader {
        Shader {
            source: ShaderSource::Glsl(glsl.to_string()),
            stage,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_spirv(&self, macros: Option<&[String]>) -> Result<Vec<u32>, ShaderError> {
        match self.source {
            ShaderSource::Spirv(ref bytes) => Ok(bytes.clone()),
            ShaderSource::Glsl(ref source) => glsl_to_spirv(&source, self.stage, macros),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_spirv_shader(&self, macros: Option<&[String]>) -> Result<Shader, ShaderError> {
        Ok(Shader {
            source: ShaderSource::Spirv(self.get_spirv(macros)?),
            stage: self.stage,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn reflect_layout(&self, enforce_bevy_conventions: bool) -> Option<ShaderLayout> {
        if let ShaderSource::Spirv(ref spirv) = self.source {
            Some(ShaderLayout::from_spirv(
                spirv.as_slice(),
                enforce_bevy_conventions,
            ))
        } else {
            panic!("Cannot reflect layout of non-SpirV shader. Try compiling this shader to SpirV first using self.get_spirv_shader().");
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn reflect_layout(&self, _enforce_bevy_conventions: bool) -> Option<ShaderLayout> {
        panic!("Cannot reflect layout on wasm32.");
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

#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let shader = match ext {
                "vert" => Shader::from_glsl(ShaderStage::Vertex, std::str::from_utf8(bytes)?),
                "frag" => Shader::from_glsl(ShaderStage::Fragment, std::str::from_utf8(bytes)?),
                #[cfg(not(target_arch = "wasm32"))]
                "spv" => Shader::from_spirv(bytes)?,
                #[cfg(target_arch = "wasm32")]
                "spv" => panic!("cannot load .spv file on wasm"),
                _ => panic!("unhandled extension: {}", ext),
            };

            load_context.set_default_asset(LoadedAsset::new(shader));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["vert", "frag", "spv"]
    }
}

pub fn shader_update_system(
    mut shaders: ResMut<Assets<Shader>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    shader_events: Res<Events<AssetEvent<Shader>>>,
    mut shader_event_reader: Local<EventReader<AssetEvent<Shader>>>,
    mut pipeline_compiler: ResMut<PipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
) {
    for event in shader_event_reader.iter(&shader_events) {
        match event {
            AssetEvent::Modified { handle } => {
                if let Err(e) = pipeline_compiler.update_shader(
                    handle,
                    &mut pipelines,
                    &mut shaders,
                    &**render_resource_context,
                ) {
                    error!("Failed to update shader: {}", e);
                }
            }
            // Creating shaders on the fly is unhandled since they
            // have to exist already when assigned to a pipeline. If a
            // shader is removed the pipeline keeps using its
            // specialized version. Maybe this should be a warning?
            AssetEvent::Created { .. } | AssetEvent::Removed { .. } => (),
        }
    }
}
