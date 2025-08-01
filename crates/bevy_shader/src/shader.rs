use super::ShaderDefVal;
use alloc::borrow::Cow;
use bevy_asset::{io::Reader, Asset, AssetLoader, AssetPath, Handle, LoadContext};
use bevy_reflect::TypePath;
use core::{marker::Copy, num::NonZero};
use thiserror::Error;

#[derive(Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Debug)]
pub struct ShaderId(NonZero<u32>);

impl ShaderId {
    #[expect(
        clippy::new_without_default,
        reason = "Implementing the `Default` trait on atomic IDs would imply that two `<AtomicIdType>::default()` equal each other. By only implementing `new()`, we indicate that each atomic ID created will be unique."
    )]
    pub fn new() -> Self {
        use core::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(NonZero::<u32>::new(counter).unwrap_or_else(|| {
            panic!("The system ran out of unique `{}`s.", stringify!(ShaderId));
        }))
    }
}
impl From<ShaderId> for NonZero<u32> {
    fn from(value: ShaderId) -> Self {
        value.0
    }
}
impl From<NonZero<u32>> for ShaderId {
    fn from(value: NonZero<u32>) -> Self {
        Self(value)
    }
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error(transparent)]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[cfg(feature = "shader_format_glsl")]
    #[error("GLSL Parse Error: {0:?}")]
    GlslParse(Vec<naga::front::glsl::Error>),
    #[cfg(feature = "shader_format_spirv")]
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::WithSpan<naga::valid::ValidationError>),
}

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

/// An "unprocessed" shader. It can contain preprocessor directives.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct Shader {
    pub path: String,
    pub source: Source,
    pub import_path: ShaderImport,
    pub imports: Vec<ShaderImport>,
    // extra imports not specified in the source string
    pub additional_imports: Vec<naga_oil::compose::ImportDefinition>,
    // any shader defs that will be included when this module is used
    pub shader_defs: Vec<ShaderDefVal>,
    // we must store strong handles to our dependencies to stop them
    // from being immediately dropped if we are the only user.
    pub file_dependencies: Vec<Handle<Shader>>,
    /// Enable or disable runtime shader validation, trading safety against speed.
    ///
    /// Please read the [`ValidateShader`] docs for a discussion of the tradeoffs involved.
    pub validate_shader: ValidateShader,
}

impl Shader {
    fn preprocess(source: &str, path: &str) -> (ShaderImport, Vec<ShaderImport>) {
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

    pub fn from_wgsl(source: impl Into<Cow<'static, str>>, path: impl Into<String>) -> Shader {
        let source = source.into();
        let path = path.into();
        let (import_path, imports) = Shader::preprocess(&source, &path);
        Shader {
            path,
            imports,
            import_path,
            source: Source::Wgsl(source),
            additional_imports: Default::default(),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }

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

    pub fn from_glsl(
        source: impl Into<Cow<'static, str>>,
        stage: naga::ShaderStage,
        path: impl Into<String>,
    ) -> Shader {
        let source = source.into();
        let path = path.into();
        let (import_path, imports) = Shader::preprocess(&source, &path);
        Shader {
            path,
            imports,
            import_path,
            source: Source::Glsl(source, stage),
            additional_imports: Default::default(),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }

    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>, path: impl Into<String>) -> Shader {
        let path = path.into();
        Shader {
            path: path.clone(),
            imports: Vec::new(),
            import_path: ShaderImport::AssetPath(path),
            source: Source::SpirV(source.into()),
            additional_imports: Default::default(),
            shader_defs: Default::default(),
            file_dependencies: Default::default(),
            validate_shader: ValidateShader::Disabled,
        }
    }

    #[cfg(feature = "shader_format_wesl")]
    pub fn from_wesl(source: impl Into<Cow<'static, str>>, path: impl Into<String>) -> Shader {
        let source = source.into();
        let path = path.into();
        let (import_path, imports) = Shader::preprocess(&source, &path);

        match import_path {
            ShaderImport::AssetPath(asset_path) => {
                // Create the shader import path - always starting with "/"
                let shader_path = std::path::Path::new("/").join(&asset_path);

                // Convert to a string with forward slashes and without extension
                let import_path_str = shader_path
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");

                let import_path = ShaderImport::AssetPath(import_path_str.to_string());

                Shader {
                    path,
                    imports,
                    import_path,
                    source: Source::Wesl(source),
                    additional_imports: Default::default(),
                    shader_defs: Default::default(),
                    file_dependencies: Default::default(),
                    validate_shader: ValidateShader::Disabled,
                }
            }
            ShaderImport::Custom(_) => {
                panic!("Wesl shaders must be imported from an asset path");
            }
        }
    }

    pub fn set_import_path<P: Into<String>>(&mut self, import_path: P) {
        self.import_path = ShaderImport::Custom(import_path.into());
    }

    #[must_use]
    pub fn with_import_path<P: Into<String>>(mut self, import_path: P) -> Self {
        self.set_import_path(import_path);
        self
    }

    #[inline]
    pub fn import_path(&self) -> &ShaderImport {
        &self.import_path
    }

    pub fn imports(&self) -> impl ExactSizeIterator<Item = &ShaderImport> {
        self.imports.iter()
    }
}

impl<'a> From<&'a Shader> for naga_oil::compose::ComposableModuleDescriptor<'a> {
    fn from(shader: &'a Shader) -> Self {
        let shader_defs = shader
            .shader_defs
            .iter()
            .map(|def| match def {
                ShaderDefVal::Bool(name, b) => {
                    (name.clone(), naga_oil::compose::ShaderDefValue::Bool(*b))
                }
                ShaderDefVal::Int(name, i) => {
                    (name.clone(), naga_oil::compose::ShaderDefValue::Int(*i))
                }
                ShaderDefVal::UInt(name, i) => {
                    (name.clone(), naga_oil::compose::ShaderDefValue::UInt(*i))
                }
            })
            .collect();

        let as_name = match &shader.import_path {
            ShaderImport::AssetPath(asset_path) => Some(format!("\"{asset_path}\"")),
            ShaderImport::Custom(_) => None,
        };

        naga_oil::compose::ComposableModuleDescriptor {
            source: shader.source.as_str(),
            file_path: &shader.path,
            language: (&shader.source).into(),
            additional_imports: &shader.additional_imports,
            shader_defs,
            as_name,
        }
    }
}

impl<'a> From<&'a Shader> for naga_oil::compose::NagaModuleDescriptor<'a> {
    fn from(shader: &'a Shader) -> Self {
        naga_oil::compose::NagaModuleDescriptor {
            source: shader.source.as_str(),
            file_path: &shader.path,
            shader_type: (&shader.source).into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    Wgsl(Cow<'static, str>),
    Wesl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

impl Source {
    pub fn as_str(&self) -> &str {
        match self {
            Source::Wgsl(s) | Source::Wesl(s) | Source::Glsl(s, _) => s,
            Source::SpirV(_) => panic!("spirv not yet implemented"),
        }
    }
}

impl From<&Source> for naga_oil::compose::ShaderLanguage {
    fn from(value: &Source) -> Self {
        match value {
            Source::Wgsl(_) => naga_oil::compose::ShaderLanguage::Wgsl,
            #[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
            Source::Glsl(_, _) => naga_oil::compose::ShaderLanguage::Glsl,
            #[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
            Source::Glsl(_, _) => panic!(
                "GLSL is not supported in this configuration; use the feature `shader_format_glsl`"
            ),
            Source::SpirV(_) => panic!("spirv not yet implemented"),
            Source::Wesl(_) => panic!("wesl not yet implemented"),
        }
    }
}

impl From<&Source> for naga_oil::compose::ShaderType {
    fn from(value: &Source) -> Self {
        match value {
            Source::Wgsl(_) => naga_oil::compose::ShaderType::Wgsl,
            #[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
            Source::Glsl(_, shader_stage) => match shader_stage {
                naga::ShaderStage::Vertex => naga_oil::compose::ShaderType::GlslVertex,
                naga::ShaderStage::Fragment => naga_oil::compose::ShaderType::GlslFragment,
                naga::ShaderStage::Compute => panic!("glsl compute not yet implemented"),
                naga::ShaderStage::Task => panic!("task shaders not yet implemented"),
                naga::ShaderStage::Mesh => panic!("mesh shaders not yet implemented"),
            },
            #[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
            Source::Glsl(_, _) => panic!(
                "GLSL is not supported in this configuration; use the feature `shader_format_glsl`"
            ),
            Source::SpirV(_) => panic!("spirv not yet implemented"),
            Source::Wesl(_) => panic!("wesl not yet implemented"),
        }
    }
}

#[derive(Default)]
pub struct ShaderLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ShaderLoaderError {
    #[error("Could not load shader: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse shader: {0}")]
    Parse(#[from] alloc::string::FromUtf8Error),
}

/// Settings for loading shaders.
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct ShaderSettings {
    /// The `#define` specified for this shader.
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
        let ext = load_context.path().extension().unwrap().to_str().unwrap();
        let path = load_context.asset_path().to_string();
        // On windows, the path will inconsistently use \ or /.
        // TODO: remove this once AssetPath forces cross-platform "slash" consistency. See #10511
        let path = path.replace(std::path::MAIN_SEPARATOR, "/");
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        if ext != "wgsl" && !settings.shader_defs.is_empty() {
            tracing::warn!(
                "Tried to load a non-wgsl shader with shader defs, this isn't supported: \
                    The shader defs will be ignored."
            );
        }
        let mut shader = match ext {
            "spv" => Shader::from_spirv(bytes, load_context.path().to_string_lossy()),
            "wgsl" => Shader::from_wgsl_with_defs(
                String::from_utf8(bytes)?,
                path,
                settings.shader_defs.clone(),
            ),
            "vert" => Shader::from_glsl(String::from_utf8(bytes)?, naga::ShaderStage::Vertex, path),
            "frag" => {
                Shader::from_glsl(String::from_utf8(bytes)?, naga::ShaderStage::Fragment, path)
            }
            "comp" => {
                Shader::from_glsl(String::from_utf8(bytes)?, naga::ShaderStage::Compute, path)
            }
            #[cfg(feature = "shader_format_wesl")]
            "wesl" => Shader::from_wesl(String::from_utf8(bytes)?, path),
            _ => panic!("unhandled extension: {ext}"),
        };

        // collect and store file dependencies
        for import in &shader.imports {
            if let ShaderImport::AssetPath(asset_path) = import {
                shader.file_dependencies.push(load_context.load(asset_path));
            }
        }
        Ok(shader)
    }

    fn extensions(&self) -> &[&str] {
        &["spv", "wgsl", "vert", "frag", "comp", "wesl"]
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    AssetPath(String),
    Custom(String),
}

impl ShaderImport {
    pub fn module_name(&self) -> Cow<'_, String> {
        match self {
            ShaderImport::AssetPath(s) => Cow::Owned(format!("\"{s}\"")),
            ShaderImport::Custom(s) => Cow::Borrowed(s),
        }
    }
}

/// A reference to a shader asset.
pub enum ShaderRef {
    /// Use the "default" shader for the current context.
    Default,
    /// A handle to a shader stored in the [`Assets<Shader>`](bevy_asset::Assets) resource
    Handle(Handle<Shader>),
    /// An asset path leading to a shader
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
