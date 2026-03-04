use crate::compiler::ShaderLanguage;
use crate::shader::{Shader, ShaderImport};
use crate::ShaderDefVal;

/// The result of composing a shader: the final source ready for compilation.
#[derive(Debug, Clone)]
pub enum ComposedSource {
    /// Text source ready for compilation.
    Text {
        /// The fully composed source code.
        code: String,
        /// The language of the composed source.
        language: ShaderLanguage,
    },
    /// A naga module produced by `naga_oil` composition.
    Naga(Box<naga::Module>),
}

/// Error type for shader import resolution / composition failures.
#[derive(Debug, thiserror::Error)]
#[error("Shader compose error: {message}")]
pub struct ShaderComposeError {
    /// A human-readable description of the error.
    pub message: String,
}

/// Trait for resolving shader imports and composing final shader source.
pub trait ShaderImportResolver: Send + Sync + 'static {
    /// Extract import information from shader source code.
    ///
    /// Returns `(import_path, imports)` — the path this shader exports,
    /// and the list of imports it depends on.
    fn extract_imports<'a>(
        &self,
        source: &'a str,
        path: &'a str,
    ) -> (ShaderImport, Vec<ShaderImport>);

    /// Register a shader as available for import by other shaders.
    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderComposeError>;

    /// Remove a previously registered import.
    fn remove_import(&mut self, import_path: &str);

    /// Check if a module with the given import name is already registered.
    fn contains_module(&self, module_name: &str) -> bool;

    /// Compose a final shader source from a root shader and its resolved imports,
    /// applying the given shader definitions.
    fn compose(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ComposedSource, ShaderComposeError>;
}

/// The default import resolver backed by `naga_oil`'s Composer.
///
/// This handles WGSL and GLSL shader import resolution and composition.
pub struct NagaOilImportResolver {
    /// The underlying `naga_oil` Composer.
    pub composer: naga_oil::compose::Composer,
}

impl NagaOilImportResolver {
    /// Create a new resolver with the given naga capabilities.
    pub fn new(capabilities: naga::valid::Capabilities, validating: bool) -> Self {
        let composer = if validating {
            naga_oil::compose::Composer::default()
        } else {
            naga_oil::compose::Composer::non_validating()
        };
        let composer = composer.with_capabilities(capabilities);
        Self { composer }
    }
}

impl ShaderImportResolver for NagaOilImportResolver {
    fn extract_imports<'a>(
        &self,
        source: &'a str,
        path: &'a str,
    ) -> (ShaderImport, Vec<ShaderImport>) {
        let (import_path, imports, _) = naga_oil::compose::get_preprocessor_data(source);

        let import_path = import_path
            .map(ShaderImport::Custom)
            .unwrap_or_else(|| ShaderImport::AssetPath(path.to_owned()));

        let imports = imports
            .into_iter()
            .map(|import| {
                if import.import.starts_with('\"') {
                    let import = import
                        .import
                        .chars()
                        .skip(1)
                        .take_while(|c| *c != '\"')
                        .collect();
                    ShaderImport::AssetPath(import)
                } else {
                    ShaderImport::Custom(import.import)
                }
            })
            .collect();

        (import_path, imports)
    }

    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderComposeError> {
        if let Err(e) = self.composer.add_composable_module(shader.into()) {
            return Err(ShaderComposeError {
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

    fn compose(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ComposedSource, ShaderComposeError> {
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

        match self
            .composer
            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                shader_defs,
                ..shader.into()
            }) {
            Ok(naga_module) => Ok(ComposedSource::Naga(Box::new(naga_module))),
            Err(e) => Err(ShaderComposeError {
                message: e.emit_to_string(&self.composer),
            }),
        }
    }
}

/// Import resolver for WESL.
#[cfg(feature = "shader_format_wesl")]
pub struct WeslImportResolver {
    /// Stored shader sources keyed by module path.
    sources: std::collections::HashMap<wesl::syntax::ModulePath, String>,
}

#[cfg(feature = "shader_format_wesl")]
impl WeslImportResolver {
    /// Create a new WESL import resolver.
    pub fn new() -> Self {
        Self {
            sources: std::collections::HashMap::new(),
        }
    }
}

#[cfg(feature = "shader_format_wesl")]
impl Default for WeslImportResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "shader_format_wesl")]
impl ShaderImportResolver for WeslImportResolver {
    fn extract_imports<'a>(
        &self,
        _source: &'a str,
        path: &'a str,
    ) -> (ShaderImport, Vec<ShaderImport>) {
        // WESL resolves imports lazily during compilation, so we don't
        // need to enumerate them upfront. Thus return the asset path as the
        // import path with no declared imports.
        (ShaderImport::AssetPath(path.to_owned()), vec![])
    }

    fn add_import(&mut self, shader: &Shader) -> Result<(), ShaderComposeError> {
        let ShaderImport::AssetPath(path) = &shader.import_path else {
            return Err(ShaderComposeError {
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

    fn compose(
        &mut self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
    ) -> Result<ComposedSource, ShaderComposeError> {
        let ShaderImport::AssetPath(path) = &shader.import_path else {
            return Err(ShaderComposeError {
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
        .map_err(|e| ShaderComposeError {
            message: format!("{e}"),
        })?;

        Ok(ComposedSource::Text {
            code: compiled.to_string(),
            language: ShaderLanguage::Wgsl,
        })
    }
}

/// Internal [`wesl::Resolver`] adapter that resolves from stored shader sources.
#[cfg(feature = "shader_format_wesl")]
struct SourceMapResolver<'a> {
    sources: &'a std::collections::HashMap<wesl::syntax::ModulePath, String>,
}

#[cfg(feature = "shader_format_wesl")]
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
