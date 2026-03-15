//! Abstract shader compiler trait and types
//!
//! Default [`ShaderCompiler`] implementations:
/// - [`NagaOilCompiler`] for WGSL
/// - [`NagaOilCompiler`] for GLSL
/// - [`WeslCompiler`] for WESL (requires `shader_format_wesl` feature)
/// - [`SpirVPassthroughCompiler`] for SPIR-V (requires `shader_format_spirv` feature)
mod naga_compiler;
pub use naga_compiler::*;

#[cfg(feature = "shader_format_wesl")]
mod wesl_compiler;
#[cfg(feature = "shader_format_wesl")]
pub use wesl_compiler::*;

use crate::shader::Shader;
use crate::ShaderDefVal;

/// Identifies the language of a shader source.
///
/// Plugin-defined languages can use [`ShaderLanguage::Custom`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderLanguage {
    /// WebGPU Shading Language.
    Wgsl,
    /// OpenGL Shading Language.
    Glsl,
    /// WebGPU Extended Shading Language.
    #[cfg(feature = "shader_format_wesl")]
    Wesl,
    /// Pre-compiled SPIR-V binary.
    #[cfg(feature = "shader_format_spirv")]
    SpirV,
    /// A user-defined or plugin-provided language.
    ///
    /// The string should be a unique identifier for the language.
    Custom(&'static str),
}

impl core::fmt::Display for ShaderLanguage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShaderLanguage::Wgsl => write!(f, "wgsl"),
            ShaderLanguage::Glsl => write!(f, "glsl"),
            #[cfg(feature = "shader_format_wesl")]
            ShaderLanguage::Wesl => write!(f, "wesl"),
            #[cfg(feature = "shader_format_spirv")]
            ShaderLanguage::SpirV => write!(f, "spirv"),
            ShaderLanguage::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Shader pipeline stage.
#[expect(missing_docs, reason = "Enum variants are self-explanatory")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderKind {
    Vertex,
    Fragment,
    Compute,
}

impl core::fmt::Display for ShaderKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShaderKind::Vertex => write!(f, "vertex"),
            ShaderKind::Fragment => write!(f, "fragment"),
            ShaderKind::Compute => write!(f, "compute"),
        }
    }
}

impl From<ShaderKind> for naga::ShaderStage {
    fn from(stage: ShaderKind) -> Self {
        match stage {
            ShaderKind::Vertex => naga::ShaderStage::Vertex,
            ShaderKind::Fragment => naga::ShaderStage::Fragment,
            ShaderKind::Compute => naga::ShaderStage::Compute,
        }
    }
}

impl From<naga::ShaderStage> for ShaderKind {
    fn from(stage: naga::ShaderStage) -> Self {
        match stage {
            naga::ShaderStage::Vertex => ShaderKind::Vertex,
            naga::ShaderStage::Fragment => ShaderKind::Fragment,
            naga::ShaderStage::Compute => ShaderKind::Compute,
            _ => panic!("unsupported naga shader stage: {stage:?}"),
        }
    }
}

/// The output of a shader compiler.
#[derive(Clone, Debug)]
pub enum CompiledShader {
    /// SPIR-V binary data.
    SpirV(Vec<u8>),
    /// WGSL source string.
    Wgsl(String),
    /// A naga IR module.
    #[cfg(not(feature = "decoupled_naga"))]
    Naga(Box<naga::Module>),
}

/// Error type for shader compilation failures.
#[derive(Debug, thiserror::Error)]
#[error("Shader compilation error: {message}")]
pub struct ShaderCompileError {
    /// A human-readable description of the error.
    pub message: String,
}

/// Compiles shader source into [`CompiledShader`] output.
///
/// Registered per-[`ShaderLanguage`] in [`ShaderCache`](crate::ShaderCache).
#[expect(
    unused_variables,
    reason = "The parameters here are intentionally unused by the default implementation; however, putting underscores here will result in the underscores being copied by rust-analyzer's tab completion."
)]
pub trait ShaderCompiler: Send + Sync + 'static {
    /// Register a shader module so it can be imported by other shaders.
    ///
    /// Called by [`ShaderCache`](crate::ShaderCache) when a shader is added.
    /// Implementations without an import system should return `Ok(())`.
    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderCompileError> {
        Ok(())
    }

    /// Remove a previously registered import module.
    ///
    /// Implementations without an import system should no-op.
    fn remove_import(&mut self, import_path: &str) {}

    /// Check if a module with the given import name is already registered.
    ///
    /// Implementations without an import system should return `false`.
    fn contains_module(&self, module_name: &str) -> bool {
        false
    }

    /// Compile a shader: resolve imports, preprocess, and produce final output.
    fn compile(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError>;
}

/// A passthrough [`ShaderCompiler`] for pre-compiled SPIR-V shaders.
///
/// Returns the SPIR-V binary data as-is with no import resolution or processing.
#[cfg(feature = "shader_format_spirv")]
pub struct SpirVPassthroughCompiler;

#[cfg(feature = "shader_format_spirv")]
impl ShaderCompiler for SpirVPassthroughCompiler {
    fn compile(
        &mut self,
        shader: &Shader,
        _shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError> {
        let data = shader
            .source
            .as_binary()
            .ok_or_else(|| ShaderCompileError {
                message: "SpirVPassthroughCompiler expects binary SPIR-V source".to_string(),
            })?;
        Ok(CompiledShader::SpirV(data.to_vec()))
    }
}
