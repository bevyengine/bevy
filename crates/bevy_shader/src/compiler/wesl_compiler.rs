use crate::compiler::{CompiledShader, ShaderCompileError, ShaderCompiler};
use crate::shader::{Shader, ShaderImport};
use crate::ShaderDefVal;

/// A [`ShaderCompiler`] for WESL (WebGPU Extended Shading Language).
///
/// Resolves imports, applies conditional compilation, and lowers WESL to WGSL text.
pub struct WeslCompiler {
    /// Stored shader sources keyed by module path.
    sources: std::collections::HashMap<wesl::syntax::ModulePath, String>,
}

impl WeslCompiler {
    /// Create a new WESL compiler.
    pub fn new() -> Self {
        Self {
            sources: std::collections::HashMap::new(),
        }
    }
}

impl Default for WeslCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderCompiler for WeslCompiler {
    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderCompileError> {
        let ShaderImport::AssetPath(path) = &shader.import_path else {
            return Err(ShaderCompileError {
                message: "WESL shaders must have an asset path import".to_string(),
            });
        };
        let module_path = wesl::syntax::ModulePath::from_path(path);
        self.sources
            .insert(module_path, shader.source.as_str().to_owned());
        Ok(())
    }

    fn remove_import(&mut self, import_path: &str) {
        // import_path for AssetPath is formatted as `"path"` by module_name()
        let path_str = import_path.trim_matches('"');
        let module_path = wesl::syntax::ModulePath::from_path(path_str);
        self.sources.remove(&module_path);
    }

    fn contains_module(&self, module_name: &str) -> bool {
        let path_str = module_name.trim_matches('"');
        let module_path = wesl::syntax::ModulePath::from_path(path_str);
        self.sources.contains_key(&module_path)
    }

    fn compile(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<CompiledShader, ShaderCompileError> {
        let ShaderImport::AssetPath(path) = &shader.import_path else {
            return Err(ShaderCompileError {
                message: "WESL shaders must be imported from a file".to_string(),
            });
        };

        let module_path = wesl::syntax::ModulePath::from_path(path);
        let resolver = SourceMapResolver {
            sources: &self.sources,
        };

        let mut compiler_options = wesl::CompileOptions {
            imports: true,
            condcomp: true,
            lower: true,
            ..Default::default()
        };

        for shader_def in shader_defs {
            match shader_def {
                ShaderDefVal::Bool(key, value) => {
                    compiler_options
                        .features
                        .flags
                        .insert(key.clone(), (*value).into());
                }
                _ => tracing::debug!(
                    "ShaderDefVal::Int and ShaderDefVal::UInt are not supported in wesl",
                ),
            }
        }

        let compiled = wesl::compile(
            &module_path,
            &resolver,
            &wesl::EscapeMangler,
            &compiler_options,
        )
        .map_err(|e| ShaderCompileError {
            message: format!("{e}"),
        })?;

        Ok(CompiledShader::Wgsl(compiled.to_string()))
    }
}

/// Internal [`wesl::Resolver`] adapter that resolves from stored shader sources.
struct SourceMapResolver<'a> {
    sources: &'a std::collections::HashMap<wesl::syntax::ModulePath, String>,
}

impl wesl::Resolver for SourceMapResolver<'_> {
    fn resolve_source<'a>(
        &'a self,
        path: &wesl::syntax::ModulePath,
    ) -> Result<alloc::borrow::Cow<'a, str>, wesl::ResolveError> {
        self.sources
            .get(path)
            .map(|s| alloc::borrow::Cow::Borrowed(s.as_str()))
            .ok_or_else(|| {
                wesl::ResolveError::ModuleNotFound(
                    path.clone(),
                    "Module not found in shader assets".to_string(),
                )
            })
    }
}
