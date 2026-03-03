use crate::ShaderDefVal;
use alloc::sync::Arc;

/// Identifies the language of a shader source.
///
/// Plugin-defined languages can use [`ShaderLanguage::Custom`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShaderLanguage {
    /// WebGPU Shading Language.
    Wgsl,
    /// OpenGL Shading Language.
    Glsl,
    /// Pre-compiled SPIR-V binary.
    SpirV,
    /// WebGPU Extended Shading Language.
    #[cfg(feature = "shader_format_wesl")]
    Wesl,
    /// A user-defined or plugin-provided language.
    ///
    /// The string should be a unique identifier for the language (e.g. `"hlsl"`).
    Custom(Arc<str>),
}

impl core::fmt::Display for ShaderLanguage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShaderLanguage::Wgsl => write!(f, "wgsl"),
            ShaderLanguage::Glsl => write!(f, "glsl"),
            ShaderLanguage::SpirV => write!(f, "spirv"),
            #[cfg(feature = "shader_format_wesl")]
            ShaderLanguage::Wesl => write!(f, "wesl"),
            ShaderLanguage::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Shader pipeline stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader stage.
    Vertex,
    /// Fragment shader stage.
    Fragment,
    /// Compute shader stage.
    Compute,
}

impl core::fmt::Display for ShaderStage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ShaderStage::Vertex => write!(f, "vertex"),
            ShaderStage::Fragment => write!(f, "fragment"),
            ShaderStage::Compute => write!(f, "compute"),
        }
    }
}

impl From<ShaderStage> for naga::ShaderStage {
    fn from(stage: ShaderStage) -> Self {
        match stage {
            ShaderStage::Vertex => naga::ShaderStage::Vertex,
            ShaderStage::Fragment => naga::ShaderStage::Fragment,
            ShaderStage::Compute => naga::ShaderStage::Compute,
        }
    }
}

impl From<naga::ShaderStage> for ShaderStage {
    fn from(stage: naga::ShaderStage) -> Self {
        match stage {
            naga::ShaderStage::Vertex => ShaderStage::Vertex,
            naga::ShaderStage::Fragment => ShaderStage::Fragment,
            naga::ShaderStage::Compute => ShaderStage::Compute,
            _ => panic!("unsupported naga shader stage: {stage:?}"),
        }
    }
}

/// The output of a shader compiler — an intermediate representation
/// ready to be turned into a GPU shader module.
#[derive(Clone, Debug)]
pub enum CompiledShader {
    /// SPIR-V binary data.
    SpirV(Vec<u8>),
    /// WGSL source string (native for wgpu).
    Wgsl(String),
    /// A naga IR module — the most common output from the default naga compiler.
    #[cfg(not(feature = "decoupled_naga"))]
    Naga(Box<naga::Module>),
}

/// A request to compile a shader.
pub struct CompileRequest<'a> {
    /// The shader source code or binary.
    pub source: ShaderSourceRef<'a>,
    /// Preprocessor defines to apply.
    pub shader_defs: &'a [ShaderDefVal],
    /// The pipeline stage this shader targets (e.g. vertex, fragment).
    ///
    /// `None` for stage-agnostic languages like WGSL where a single source
    /// can contain multiple entry points.
    pub stage: Option<ShaderStage>,
}

/// A reference to shader source material.
pub enum ShaderSourceRef<'a> {
    /// Text-based shader source with a language tag.
    Text {
        /// The source code.
        code: &'a str,
        /// The language of the source code.
        language: &'a ShaderLanguage,
    },
    /// Binary shader source (e.g. SPIR-V).
    Binary {
        /// The binary data.
        data: &'a [u8],
        /// The language of the binary data.
        language: &'a ShaderLanguage,
    },
    /// A naga IR module produced by the import resolver.
    Naga {
        /// The composed naga IR module.
        module: &'a naga::Module,
    },
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
/// The default implementation is [`NagaShaderCompiler`](crate::NagaShaderCompiler).
pub trait ShaderCompiler: Send + Sync + 'static {
    /// Compile a shader from its final composed source.
    fn compile(&self, request: &CompileRequest) -> Result<CompiledShader, ShaderCompileError>;
}
