use super::ShaderDefVal;
use alloc::borrow::Cow;
use bevy_asset::{io::Reader, Asset, AssetLoader, AssetPath, Handle, LoadContext};
use bevy_reflect::TypePath;
use bevy_utils::define_atomic_id;
use thiserror::Error;

fn scan_wesl_imports(
    source: &str,
    self_module_path: &wesl::syntax::ModulePath,
) -> Vec<ShaderImport> {
    use wesl::syntax::{ImportContent, ModulePath, PathOrigin};

    fn leaves(content: &ImportContent, path: ModulePath, out: &mut Vec<ModulePath>) {
        match content {
            ImportContent::Item(item) => {
                let mut full = path.clone();
                full.push(&item.ident.to_string());
                out.push(path);
                out.push(full);
            }
            ImportContent::Collection(collection) => {
                for import in collection {
                    let path = path.clone().join(import.path.iter().cloned());
                    leaves(&import.content, path, out);
                }
            }
        }
    }

    let Ok(translation_unit) = source.parse::<wesl::syntax::TranslationUnit>() else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    for statement in &translation_unit.imports {
        match &statement.path {
            Some(import_path) => {
                let path = self_module_path.join_path(import_path);
                leaves(&statement.content, path, &mut paths);
            }
            None => {
                if let ImportContent::Collection(collection) = &statement.content {
                    for import in collection {
                        let mut components = import.path.iter().cloned();
                        if let Some(package) = components.next() {
                            let path =
                                ModulePath::new(PathOrigin::Package(package), components.collect());
                            leaves(&import.content, path, &mut paths);
                        }
                    }
                }
            }
        }
    }

    let mut imports = Vec::new();
    for path in &paths {
        let path = match &path.origin {
            PathOrigin::Package(pkg) if pkg.contains('/') => Cow::Owned(ModulePath {
                origin: PathOrigin::Package(pkg.rsplit('/').next().unwrap().to_string()),
                components: path.components.clone(),
            }),
            _ => Cow::Borrowed(path),
        };
        let import = match &path.origin {
            PathOrigin::Absolute => {
                ShaderImport::AssetPath(format!("/{}", path.components.join("/")))
            }
            PathOrigin::Package(package) => ShaderImport::Custom(
                core::iter::once(package.as_str())
                    .chain(path.components.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join("::"),
            ),
            PathOrigin::Relative(_) => continue,
        };
        if !imports.contains(&import) {
            imports.push(import);
        }
    }
    imports
}

define_atomic_id!(ShaderId);

/// Describes whether or not to perform runtime checks on shaders.
/// Runtime checks can be enabled for safety at the cost of speed.
/// By default no runtime checks will be performed.
///
/// # Panics
/// Because no runtime checks are performed for spirv,
/// enabling `ValidateShader` for spirv will cause a panic
#[derive(Clone, Debug, Default)]
pub enum ValidateShader {
    #[default]
    /// No runtime checks for soundness (e.g. bound checking) are performed.
    ///
    /// This is suitable for trusted shaders, written by your program or dependencies you trust.
    Disabled,
    /// Enable's runtime checks for soundness (e.g. bound checking).
    ///
    /// While this can have a meaningful impact on performance,
    /// this setting should *always* be enabled when loading untrusted shaders.
    /// This might occur if you are creating a shader playground, running user-generated shaders
    /// (as in `VRChat`), or writing a web browser in Bevy.
    Enabled,
}

/// An "unprocessed" shader. It can contain imports and conditional
/// compilation attributes.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct Shader {
    /// The asset path of the shader.
    pub path: String,
    /// The raw source code of the shader.
    pub source: Source,
    /// The path from which this shader can be imported by other shaders.
    pub import_path: ShaderImport,
    /// The import paths this shader depends on.
    pub imports: Vec<ShaderImport>,
    /// Any shader defs that should be included when this module is used.
    pub shader_defs: Vec<ShaderDefVal>,
    /// Strong handles to this shader's dependencies, to prevent them
    /// from being immediately dropped if this shader is the only user.
    pub file_dependencies: Vec<Handle<Shader>>,
    /// Enable or disable runtime shader validation, trading safety against speed.
    ///
    /// Please read the [`ValidateShader`] docs for a discussion of the tradeoffs involved.
    pub validate_shader: ValidateShader,
}

impl Shader {
    /// Creates a new WGSL shader.
    pub fn from_wgsl(source: impl Into<Cow<'static, str>>, path: impl Into<String>) -> Shader {
        let source = source.into();
        let path = path.into();
        Shader {
            import_path: ShaderImport::AssetPath(path.clone()),
            path,
            imports: Vec::new(),
            source: Source::Wgsl(source),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }

    /// Creates a new WGSL shader with some given shader defs.
    pub fn from_wgsl_with_defs(
        source: impl Into<Cow<'static, str>>,
        path: impl Into<String>,
        shader_defs: Vec<ShaderDefVal>,
    ) -> Shader {
        Self {
            shader_defs,
            ..Self::from_wgsl(source, path)
        }
    }

    /// Creates a new SPIR-V shader.
    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>, path: impl Into<String>) -> Shader {
        let path = path.into();
        Shader {
            path: path.clone(),
            imports: Vec::new(),
            import_path: ShaderImport::AssetPath(path),
            source: Source::SpirV(source.into()),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }

    /// Creates a new Wesl shader.
    pub fn from_wesl(source: impl Into<Cow<'static, str>>, path: impl Into<String>) -> Shader {
        let source = source.into();
        let path = path.into();

        let import_path = match path.strip_prefix("embedded://") {
            Some(embedded_path) => ShaderImport::Custom(
                std::path::Path::new(embedded_path)
                    .with_extension("")
                    .to_string_lossy()
                    .split('/')
                    .filter(|component| !component.is_empty())
                    .collect::<Vec<_>>()
                    .join("::"),
            ),
            None => {
                // Create the shader import path - always starting with "/"
                let shader_path = std::path::Path::new("/").join(&path);

                // Convert to a string with forward slashes and without extension
                let import_path_str = shader_path
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");

                ShaderImport::AssetPath(import_path_str.to_string())
            }
        };

        let imports = crate::shader_cache::wesl_module_path(&import_path)
            .map(|module_path| scan_wesl_imports(&source, &module_path))
            .unwrap_or_default();

        Shader {
            path,
            imports,
            import_path,
            source: Source::Wesl(source),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }
}

/// Raw shader source code.
#[expect(missing_docs, reason = "The variants are self-explanatory.")]
#[derive(Debug, Clone)]
pub enum Source {
    Wgsl(Cow<'static, str>),
    Wesl(Cow<'static, str>),
    SpirV(Cow<'static, [u8]>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

impl Source {
    /// The underlying source code string, unless it is SPIR-V.
    pub fn as_str(&self) -> &str {
        match self {
            Source::Wgsl(s) | Source::Wesl(s) => s,
            Source::SpirV(_) => panic!("spirv not yet implemented"),
        }
    }
}

/// The [`AssetLoader`] responsible for loading unprocessed shader assets.
#[derive(Default, TypePath)]
pub struct ShaderLoader;

/// An error encountered while loading a shader's source.
#[non_exhaustive]
#[derive(Debug, Error)]
#[expect(missing_docs, reason = "The variants are self-explanatory.")]
pub enum ShaderLoaderError {
    #[error("Could not load shader: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse shader: {0}")]
    Parse(#[from] alloc::string::FromUtf8Error),
}

/// Settings for loading shaders.
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct ShaderSettings {
    /// The shader defs to apply when this shader is loaded.
    pub shader_defs: Vec<ShaderDefVal>,
}

impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ShaderSettings;
    type Error = ShaderLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Shader, Self::Error> {
        let ext = load_context
            .path()
            .path()
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let path = load_context.path().to_string();
        // On windows, the path will inconsistently use \ or /.
        // TODO: remove this once AssetPath forces cross-platform "slash" consistency. See #10511
        let path = path.replace(std::path::MAIN_SEPARATOR, "/");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        if ext != "wesl" && !settings.shader_defs.is_empty() {
            tracing::warn!(
                "Tried to load a non-wesl shader with shader defs, this isn't supported: \
                    The shader defs will be ignored."
            );
        }
        let mut shader = match ext.as_str() {
            "spv" => Shader::from_spirv(bytes, load_context.path().path().to_string_lossy()),
            "wgsl" => Shader::from_wgsl(String::from_utf8(bytes)?, path),
            "wesl" => {
                let mut shader = Shader::from_wesl(String::from_utf8(bytes)?, path);
                shader.shader_defs = settings.shader_defs.clone();
                shader
            }
            _ => panic!("unhandled extension: {ext}"),
        };

        // collect and store file dependencies
        match ext.as_str() {
            "wesl" => {
                let candidates: Vec<String> = shader
                    .imports
                    .iter()
                    .filter_map(|import| match import {
                        ShaderImport::AssetPath(asset_path) => {
                            Some(format!("{}.{ext}", asset_path.trim_start_matches('/')))
                        }
                        ShaderImport::Custom(_) => None,
                    })
                    .collect();
                for file_path in candidates {
                    if load_context
                        .read_asset_bytes(AssetPath::from(file_path.clone()))
                        .await
                        .is_ok()
                    {
                        shader
                            .file_dependencies
                            .push(load_context.load(AssetPath::from(file_path)));
                    }
                }
            }
            _ => {
                for import in &shader.imports {
                    if let ShaderImport::AssetPath(asset_path) = import {
                        shader.file_dependencies.push(load_context.load(asset_path));
                    }
                }
            }
        }
        Ok(shader)
    }

    fn extensions(&self) -> &[&str] {
        &["spv", "wgsl", "wesl"]
    }
}

/// A shader import, described as either an asset path or an import path.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    /// An asset path to a shader.
    AssetPath(String),
    /// An import path from which a shader may be imported.
    Custom(String),
}

/// A reference to a shader asset.
#[derive(Default)]
pub enum ShaderRef {
    /// Use the "default" shader for the current context.
    #[default]
    Default,
    /// A handle to a shader stored in the [`Assets<Shader>`](bevy_asset::Assets) resource.
    Handle(Handle<Shader>),
    /// An asset path leading to a shader.
    Path(AssetPath<'static>),
}

impl From<Handle<Shader>> for ShaderRef {
    fn from(handle: Handle<Shader>) -> Self {
        Self::Handle(handle)
    }
}

impl From<AssetPath<'static>> for ShaderRef {
    fn from(path: AssetPath<'static>) -> Self {
        Self::Path(path)
    }
}

impl From<&'static str> for ShaderRef {
    fn from(path: &'static str) -> Self {
        Self::Path(AssetPath::from(path))
    }
}
