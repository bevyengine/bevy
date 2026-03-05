use crate::compiler::{CompiledShader, ShaderCompileError, ShaderCompiler};
use crate::shader::Shader;
use crate::ShaderDefVal;

/// The default [`ShaderCompiler`] for WGSL and GLSL, backed by `naga_oil`.
///
/// Handles import resolution, composition, and produces naga IR
/// (or WGSL text when the `decoupled_naga` feature is enabled).
///
/// A single instance is typically shared between WGSL and GLSL
/// since `naga_oil`'s [`Composer`](naga_oil::compose::Composer) handles both.
pub struct NagaOilCompiler {
    /// The underlying `naga_oil` Composer.
    pub composer: naga_oil::compose::Composer,
    #[cfg(feature = "decoupled_naga")]
    capabilities: naga::valid::Capabilities,
}

impl NagaOilCompiler {
    /// Create a new compiler with the given naga capabilities.
    pub fn new(capabilities: naga::valid::Capabilities, validating: bool) -> Self {
        let composer = if validating {
            naga_oil::compose::Composer::default()
        } else {
            naga_oil::compose::Composer::non_validating()
        };
        let composer = composer.with_capabilities(capabilities);
        Self {
            composer,
            #[cfg(feature = "decoupled_naga")]
            capabilities,
        }
    }
}

impl ShaderCompiler for NagaOilCompiler {
    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderCompileError> {
        if let Err(e) = self.composer.add_composable_module(shader.into()) {
            return Err(ShaderCompileError {
                message: e.emit_to_string(&self.composer),
            });
        }
        Ok(())
    }

    fn remove_import(&mut self, import_path: &str) {
        self.composer.remove_composable_module(import_path);
    }

    fn contains_module(&self, module_name: &str) -> bool {
        self.composer.contains_module(module_name)
    }

    fn compile(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError> {
        let shader_defs: std::collections::HashMap<String, naga_oil::compose::ShaderDefValue> =
            shader_defs
                .iter()
                .chain(shader.shader_defs.iter())
                .map(|def| match def.clone() {
                    ShaderDefVal::Bool(k, v) => (k, naga_oil::compose::ShaderDefValue::Bool(v)),
                    ShaderDefVal::Int(k, v) => (k, naga_oil::compose::ShaderDefValue::Int(v)),
                    ShaderDefVal::UInt(k, v) => (k, naga_oil::compose::ShaderDefValue::UInt(v)),
                })
                .collect();

        let naga_module = self
            .composer
            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                shader_defs,
                ..shader.into()
            })
            .map_err(|e| ShaderCompileError {
                message: e.emit_to_string(&self.composer),
            })?;

        #[cfg(not(feature = "decoupled_naga"))]
        {
            Ok(CompiledShader::Naga(Box::new(naga_module)))
        }
        #[cfg(feature = "decoupled_naga")]
        {
            let mut validator =
                naga::valid::Validator::new(naga::valid::ValidationFlags::all(), self.capabilities);
            let module_info = validator
                .validate(&naga_module)
                .map_err(|e| ShaderCompileError {
                    message: format!("{e:?}"),
                })?;
            let wgsl = naga::back::wgsl::write_string(
                &naga_module,
                &module_info,
                naga::back::wgsl::WriterFlags::empty(),
            )
            .map_err(|e| ShaderCompileError {
                message: format!("{e:?}"),
            })?;
            Ok(CompiledShader::Wgsl(wgsl))
        }
    }
}
