use naga::valid::Capabilities;

use crate::compiler::{
    CompileRequest, CompiledShader, ShaderCompileError, ShaderCompiler, ShaderSourceRef,
};

/// The default [`ShaderCompiler`](crate::ShaderCompiler) backed by naga.
///
/// Handles naga IR passthrough (or WGSL emission with `decoupled_naga`),
/// WGSL text passthrough, and SPIR-V binary passthrough.
pub struct NagaShaderCompiler {
    #[cfg_attr(
        not(feature = "decoupled_naga"),
        expect(
            dead_code,
            reason = "capabilities is only read by the naga validator when `decoupled_naga` is enabled"
        )
    )]
    capabilities: Capabilities,
}

impl NagaShaderCompiler {
    /// Create a new `NagaShaderCompiler` with the given device capabilities.
    ///
    /// Capabilities are used by the naga validator when `decoupled_naga` is enabled.
    pub fn new(capabilities: Capabilities) -> Self {
        Self { capabilities }
    }
}

impl Default for NagaShaderCompiler {
    fn default() -> Self {
        Self::new(Capabilities::empty())
    }
}

impl ShaderCompiler for NagaShaderCompiler {
    fn compile(&self, request: &CompileRequest) -> Result<CompiledShader, ShaderCompileError> {
        match &request.source {
            ShaderSourceRef::Naga { module } => {
                #[cfg(not(feature = "decoupled_naga"))]
                {
                    Ok(CompiledShader::Naga(Box::new((*module).clone())))
                }
                #[cfg(feature = "decoupled_naga")]
                {
                    let mut validator = naga::valid::Validator::new(
                        naga::valid::ValidationFlags::all(),
                        self.capabilities,
                    );
                    let module_info = validator.validate(module).map_err(|e| ShaderCompileError {
                        message: format!("{e:?}"),
                    })?;
                    let wgsl = naga::back::wgsl::write_string(
                        module,
                        &module_info,
                        naga::back::wgsl::WriterFlags::empty(),
                    )
                    .map_err(|e| ShaderCompileError {
                        message: format!("{e:?}"),
                    })?;
                    Ok(CompiledShader::Wgsl(wgsl))
                }
            }
            ShaderSourceRef::Text { code, language } => match language {
                crate::compiler::ShaderLanguage::Wgsl => Ok(CompiledShader::Wgsl((*code).to_owned())),
                other => Err(ShaderCompileError {
                    message: format!(
                        "NagaShaderCompiler does not support compiling text source in language: {other}"
                    ),
                }),
            },
            ShaderSourceRef::Binary { data, language } => match language {
                crate::compiler::ShaderLanguage::SpirV => Ok(CompiledShader::SpirV(data.to_vec())),
                other => Err(ShaderCompileError {
                    message: format!(
                        "NagaShaderCompiler does not support compiling binary source in language: {other}"
                    ),
                }),
            },
        }
    }
}
