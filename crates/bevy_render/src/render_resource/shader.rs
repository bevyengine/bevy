use super::ShaderDefVal;
use crate::define_atomic_id;
use bevy_asset::{AssetLoader, AssetPath, Handle, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::{tracing::error, BoxedFuture, HashMap};
use naga::{back::wgsl::WriterFlags, valid::Capabilities, valid::ModuleInfo, Module};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{borrow::Cow, marker::Copy, ops::Deref, path::PathBuf, str::FromStr};
use thiserror::Error;
use wgpu::{util::make_spirv, Features, ShaderModuleDescriptor, ShaderSource};

define_atomic_id!(ShaderId);

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error(transparent)]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[error("GLSL Parse Error: {0:?}")]
    GlslParse(Vec<naga::front::glsl::Error>),
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::WithSpan<naga::valid::ValidationError>),
}
/// A shader, as defined by its [`ShaderSource`] and [`ShaderStage`](naga::ShaderStage)
/// This is an "unprocessed" shader. It can contain preprocessor directives.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    source: Source,
    import_path: Option<ShaderImport>,
    imports: Vec<ShaderImport>,
}

impl Shader {
    pub fn from_wgsl(source: impl Into<Cow<'static, str>>) -> Shader {
        let source = source.into();
        let shader_imports = SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source);
        Shader {
            imports: shader_imports.imports,
            import_path: shader_imports.import_path,
            source: Source::Wgsl(source),
        }
    }

    pub fn from_glsl(source: impl Into<Cow<'static, str>>, stage: naga::ShaderStage) -> Shader {
        let source = source.into();
        let shader_imports = SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source);
        Shader {
            imports: shader_imports.imports,
            import_path: shader_imports.import_path,
            source: Source::Glsl(source, stage),
        }
    }

    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>) -> Shader {
        Shader {
            imports: Vec::new(),
            import_path: None,
            source: Source::SpirV(source.into()),
        }
    }

    pub fn set_import_path<P: Into<String>>(&mut self, import_path: P) {
        self.import_path = Some(ShaderImport::Custom(import_path.into()));
    }

    #[must_use]
    pub fn with_import_path<P: Into<String>>(mut self, import_path: P) -> Self {
        self.set_import_path(import_path);
        self
    }

    #[inline]
    pub fn import_path(&self) -> Option<&ShaderImport> {
        self.import_path.as_ref()
    }

    pub fn imports(&self) -> impl ExactSizeIterator<Item = &ShaderImport> {
        self.imports.iter()
    }
}

#[derive(Debug, Clone)]
pub enum Source {
    Wgsl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

/// A processed [Shader]. This cannot contain preprocessor directions. It must be "ready to compile"
#[derive(PartialEq, Eq, Debug)]
pub enum ProcessedShader {
    Wgsl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
}

impl ProcessedShader {
    pub fn get_wgsl_source(&self) -> Option<&str> {
        if let ProcessedShader::Wgsl(source) = self {
            Some(source)
        } else {
            None
        }
    }
    pub fn get_glsl_source(&self) -> Option<&str> {
        if let ProcessedShader::Glsl(source, _stage) = self {
            Some(source)
        } else {
            None
        }
    }

    pub fn reflect(&self, features: Features) -> Result<ShaderReflection, ShaderReflectError> {
        let module = match &self {
            // TODO: process macros here
            ProcessedShader::Wgsl(source) => naga::front::wgsl::parse_str(source)?,
            ProcessedShader::Glsl(source, shader_stage) => {
                let mut parser = naga::front::glsl::Parser::default();
                parser
                    .parse(&naga::front::glsl::Options::from(*shader_stage), source)
                    .map_err(ShaderReflectError::GlslParse)?
            }
            ProcessedShader::SpirV(source) => naga::front::spv::parse_u8_slice(
                source,
                &naga::front::spv::Options {
                    adjust_coordinate_space: false,
                    ..naga::front::spv::Options::default()
                },
            )?,
        };
        const CAPABILITIES: &[(Features, Capabilities)] = &[
            (Features::PUSH_CONSTANTS, Capabilities::PUSH_CONSTANT),
            (Features::SHADER_F64, Capabilities::FLOAT64),
            (
                Features::SHADER_PRIMITIVE_INDEX,
                Capabilities::PRIMITIVE_INDEX,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            ),
            (
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::SAMPLER_NON_UNIFORM_INDEXING,
            ),
            (
                Features::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
                Capabilities::UNIFORM_BUFFER_AND_STORAGE_TEXTURE_ARRAY_NON_UNIFORM_INDEXING,
            ),
        ];
        let mut capabilities = Capabilities::empty();
        for (feature, capability) in CAPABILITIES {
            if features.contains(*feature) {
                capabilities |= *capability;
            }
        }
        let module_info =
            naga::valid::Validator::new(naga::valid::ValidationFlags::default(), capabilities)
                .validate(&module)?;

        Ok(ShaderReflection {
            module,
            module_info,
        })
    }

    pub fn get_module_descriptor(
        &self,
        features: Features,
    ) -> Result<ShaderModuleDescriptor, AsModuleDescriptorError> {
        Ok(ShaderModuleDescriptor {
            label: None,
            source: match self {
                ProcessedShader::Wgsl(source) => {
                    #[cfg(debug_assertions)]
                    // Parse and validate the shader early, so that (e.g. while hot reloading) we can
                    // display nicely formatted error messages instead of relying on just displaying the error string
                    // returned by wgpu upon creating the shader module.
                    let _ = self.reflect(features)?;

                    ShaderSource::Wgsl(source.clone())
                }
                ProcessedShader::Glsl(_source, _stage) => {
                    let reflection = self.reflect(features)?;
                    // TODO: it probably makes more sense to convert this to spirv, but as of writing
                    // this comment, naga's spirv conversion is broken
                    let wgsl = reflection.get_wgsl()?;
                    ShaderSource::Wgsl(wgsl.into())
                }
                ProcessedShader::SpirV(source) => make_spirv(source),
            },
        })
    }
}

#[derive(Error, Debug)]
pub enum AsModuleDescriptorError {
    #[error(transparent)]
    ShaderReflectError(#[from] ShaderReflectError),
    #[error(transparent)]
    WgslConversion(#[from] naga::back::wgsl::Error),
    #[error(transparent)]
    SpirVConversion(#[from] naga::back::spv::Error),
}

pub struct ShaderReflection {
    pub module: Module,
    pub module_info: ModuleInfo,
}

impl ShaderReflection {
    pub fn get_spirv(&self) -> Result<Vec<u32>, naga::back::spv::Error> {
        naga::back::spv::write_vec(
            &self.module,
            &self.module_info,
            &naga::back::spv::Options {
                flags: naga::back::spv::WriterFlags::empty(),
                ..naga::back::spv::Options::default()
            },
            None,
        )
    }

    pub fn get_wgsl(&self) -> Result<String, naga::back::wgsl::Error> {
        naga::back::wgsl::write_string(&self.module, &self.module_info, WriterFlags::EXPLICIT_TYPES)
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

            let mut shader = match ext {
                "spv" => Shader::from_spirv(Vec::from(bytes)),
                "wgsl" => Shader::from_wgsl(String::from_utf8(Vec::from(bytes))?),
                "vert" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Vertex,
                ),
                "frag" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Fragment,
                ),
                "comp" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Compute,
                ),
                _ => panic!("unhandled extension: {ext}"),
            };

            let shader_imports = SHADER_IMPORT_PROCESSOR.get_imports(&shader);
            if shader_imports.import_path.is_some() {
                shader.import_path = shader_imports.import_path;
            } else {
                shader.import_path = Some(ShaderImport::AssetPath(
                    load_context.path().to_string_lossy().to_string(),
                ));
            }
            let mut asset = LoadedAsset::new(shader);
            for import in shader_imports.imports {
                if let ShaderImport::AssetPath(asset_path) = import {
                    let path = PathBuf::from_str(&asset_path)?;
                    asset.add_dependency(path.into());
                }
            }

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["spv", "wgsl", "vert", "frag", "comp"]
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ProcessShaderError {
    #[error("Too many '# endif' lines. Each endif should be preceded by an if statement.")]
    TooManyEndIfs,
    #[error(
        "Not enough '# endif' lines. Each if statement should be followed by an endif statement."
    )]
    NotEnoughEndIfs,
    #[error("This Shader's format does not support imports.")]
    ShaderFormatDoesNotSupportImports,
    #[error("Unresolved import: {0:?}.")]
    UnresolvedImport(ShaderImport),
    #[error("The shader import {0:?} does not match the source file type. Support for this might be added in the future.")]
    MismatchedImportFormat(ShaderImport),
    #[error("Unknown shader def operator: '{operator}'")]
    UnknownShaderDefOperator { operator: String },
    #[error("Unknown shader def: '{shader_def_name}'")]
    UnknownShaderDef { shader_def_name: String },
    #[error(
        "Invalid shader def comparison for '{shader_def_name}': expected {expected}, got {value}"
    )]
    InvalidShaderDefComparisonValue {
        shader_def_name: String,
        expected: String,
        value: String,
    },
    #[error("Invalid shader def definition for '{shader_def_name}': {value}")]
    InvalidShaderDefDefinitionValue {
        shader_def_name: String,
        value: String,
    },
}

pub struct ShaderImportProcessor {
    import_asset_path_regex: Regex,
    import_custom_path_regex: Regex,
    define_import_path_regex: Regex,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    AssetPath(String),
    Custom(String),
}

impl Default for ShaderImportProcessor {
    fn default() -> Self {
        Self {
            import_asset_path_regex: Regex::new(r#"^\s*#\s*import\s+"(.+)""#).unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+(.+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+(.+)").unwrap(),
        }
    }
}

#[derive(Default)]
pub struct ShaderImports {
    imports: Vec<ShaderImport>,
    import_path: Option<ShaderImport>,
}

impl ShaderImportProcessor {
    pub fn get_imports(&self, shader: &Shader) -> ShaderImports {
        match &shader.source {
            Source::Wgsl(source) => self.get_imports_from_str(source),
            Source::Glsl(source, _stage) => self.get_imports_from_str(source),
            Source::SpirV(_source) => ShaderImports::default(),
        }
    }

    pub fn get_imports_from_str(&self, shader: &str) -> ShaderImports {
        let mut shader_imports = ShaderImports::default();
        for line in shader.lines() {
            if let Some(cap) = self.import_asset_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                shader_imports
                    .imports
                    .push(ShaderImport::AssetPath(import.as_str().to_string()));
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                shader_imports
                    .imports
                    .push(ShaderImport::Custom(import.as_str().to_string()));
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                let path = cap.get(1).unwrap();
                shader_imports.import_path = Some(ShaderImport::Custom(path.as_str().to_string()));
            }
        }

        shader_imports
    }
}

pub static SHADER_IMPORT_PROCESSOR: Lazy<ShaderImportProcessor> =
    Lazy::new(ShaderImportProcessor::default);

pub struct ShaderProcessor {
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    ifop_regex: Regex,
    else_ifdef_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
    define_regex: Regex,
    def_regex: Regex,
    def_regex_delimited: Regex,
}

impl Default for ShaderProcessor {
    fn default() -> Self {
        Self {
            ifdef_regex: Regex::new(r"^\s*#\s*ifdef\s*([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*ifndef\s*([\w|\d|_]+)").unwrap(),
            ifop_regex: Regex::new(r"^\s*#\s*if\s*([\w|\d|_]+)\s*([^\s]*)\s*([-\w|\d]+)").unwrap(),
            else_ifdef_regex: Regex::new(r"^\s*#\s*else\s+ifdef\s*([\w|\d|_]+)").unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
            define_regex: Regex::new(r"^\s*#\s*define\s+([\w|\d|_]+)\s*([-\w|\d]+)?").unwrap(),
            def_regex: Regex::new(r"#\s*([\w|\d|_]+)").unwrap(),
            def_regex_delimited: Regex::new(r"#\s*\{([\w|\d|_]+)\}").unwrap(),
        }
    }
}

struct Scope {
    // Is the current scope one in which we should accept new lines into the output?
    accepting_lines: bool,

    // Has this scope ever accepted lines?
    // Needs to be tracked for #else ifdef chains.
    has_accepted_lines: bool,
}

impl Scope {
    fn new(should_lines_be_accepted: bool) -> Self {
        Self {
            accepting_lines: should_lines_be_accepted,
            has_accepted_lines: should_lines_be_accepted,
        }
    }

    fn is_accepting_lines(&self) -> bool {
        self.accepting_lines
    }

    fn stop_accepting_lines(&mut self) {
        self.accepting_lines = false;
    }

    fn start_accepting_lines_if_appropriate(&mut self) {
        if !self.has_accepted_lines {
            self.has_accepted_lines = true;
            self.accepting_lines = true;
        } else {
            self.accepting_lines = false;
        }
    }
}

impl ShaderProcessor {
    pub fn process(
        &self,
        shader: &Shader,
        shader_defs: &[ShaderDefVal],
        shaders: &HashMap<Handle<Shader>, Shader>,
        import_handles: &HashMap<ShaderImport, Handle<Shader>>,
    ) -> Result<ProcessedShader, ProcessShaderError> {
        let mut shader_defs_unique =
            HashMap::<String, ShaderDefVal>::from_iter(shader_defs.iter().map(|v| match v {
                ShaderDefVal::Bool(k, _) | ShaderDefVal::Int(k, _) | ShaderDefVal::UInt(k, _) => {
                    (k.clone(), v.clone())
                }
            }));
        self.process_inner(shader, &mut shader_defs_unique, shaders, import_handles)
    }

    fn process_inner(
        &self,
        shader: &Shader,
        shader_defs_unique: &mut HashMap<String, ShaderDefVal>,
        shaders: &HashMap<Handle<Shader>, Shader>,
        import_handles: &HashMap<ShaderImport, Handle<Shader>>,
    ) -> Result<ProcessedShader, ProcessShaderError> {
        let shader_str = match &shader.source {
            Source::Wgsl(source) => source.deref(),
            Source::Glsl(source, _stage) => source.deref(),
            Source::SpirV(source) => {
                return Ok(ProcessedShader::SpirV(source.clone()));
            }
        };

        let mut scopes = vec![Scope::new(true)];
        let mut final_string = String::new();
        for line in shader_str.lines() {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();

                let current_valid = scopes.last().unwrap().is_accepting_lines();
                let has_define = shader_defs_unique.contains_key(def.as_str());

                scopes.push(Scope::new(current_valid && has_define));
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();

                let current_valid = scopes.last().unwrap().is_accepting_lines();
                let has_define = shader_defs_unique.contains_key(def.as_str());

                scopes.push(Scope::new(current_valid && !has_define));
            } else if let Some(cap) = self.ifop_regex.captures(line) {
                let def = cap.get(1).unwrap();
                let op = cap.get(2).unwrap();
                let val = cap.get(3).unwrap();

                fn act_on<T: Eq + Ord>(a: T, b: T, op: &str) -> Result<bool, ProcessShaderError> {
                    match op {
                        "==" => Ok(a == b),
                        "!=" => Ok(a != b),
                        ">" => Ok(a > b),
                        ">=" => Ok(a >= b),
                        "<" => Ok(a < b),
                        "<=" => Ok(a <= b),
                        _ => Err(ProcessShaderError::UnknownShaderDefOperator {
                            operator: op.to_string(),
                        }),
                    }
                }

                let def = shader_defs_unique.get(def.as_str()).ok_or(
                    ProcessShaderError::UnknownShaderDef {
                        shader_def_name: def.as_str().to_string(),
                    },
                )?;
                let new_scope = match def {
                    ShaderDefVal::Bool(name, def) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ProcessShaderError::InvalidShaderDefComparisonValue {
                                shader_def_name: name.clone(),
                                value: val.as_str().to_string(),
                                expected: "bool".to_string(),
                            }
                        })?;
                        act_on(*def, val, op.as_str())?
                    }
                    ShaderDefVal::Int(name, def) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ProcessShaderError::InvalidShaderDefComparisonValue {
                                shader_def_name: name.clone(),
                                value: val.as_str().to_string(),
                                expected: "int".to_string(),
                            }
                        })?;
                        act_on(*def, val, op.as_str())?
                    }
                    ShaderDefVal::UInt(name, def) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ProcessShaderError::InvalidShaderDefComparisonValue {
                                shader_def_name: name.clone(),
                                value: val.as_str().to_string(),
                                expected: "uint".to_string(),
                            }
                        })?;
                        act_on(*def, val, op.as_str())?
                    }
                };

                let current_valid = scopes.last().unwrap().is_accepting_lines();

                scopes.push(Scope::new(current_valid && new_scope));
            } else if let Some(cap) = self.else_ifdef_regex.captures(line) {
                // When should we accept the code in an
                //
                //  #else ifdef FOO
                //      <stuff>
                //  #endif
                //
                // block? Conditions:
                //  1. The parent scope is accepting lines.
                //  2. The current scope is _not_ accepting lines.
                //  3. FOO is defined.
                //  4. We haven't already accepted another #ifdef (or #else ifdef) in the current scope.

                // Condition 1
                let mut parent_accepting = true;

                if scopes.len() > 1 {
                    parent_accepting = scopes[scopes.len() - 2].is_accepting_lines();
                }

                if let Some(current) = scopes.last_mut() {
                    // Condition 2
                    let current_accepting = current.is_accepting_lines();

                    // Condition 3
                    let def = cap.get(1).unwrap();
                    let has_define = shader_defs_unique.contains_key(def.as_str());

                    if parent_accepting && !current_accepting && has_define {
                        // Condition 4: Enforced by [`Scope`].
                        current.start_accepting_lines_if_appropriate();
                    } else {
                        current.stop_accepting_lines();
                    }
                }
            } else if self.else_regex.is_match(line) {
                let mut parent_accepting = true;

                if scopes.len() > 1 {
                    parent_accepting = scopes[scopes.len() - 2].is_accepting_lines();
                }
                if let Some(current) = scopes.last_mut() {
                    // Using #else means that we only want to accept those lines in the output
                    // if the stuff before #else was _not_ accepted.
                    // That's why we stop accepting here if we were currently accepting.
                    //
                    // Why do we care about the parent scope?
                    // Because if we have something like this:
                    //
                    //  #ifdef NOT_DEFINED
                    //      // Not accepting lines
                    //      #ifdef NOT_DEFINED_EITHER
                    //          // Not accepting lines
                    //      #else
                    //          // This is now accepting lines relative to NOT_DEFINED_EITHER
                    //          <stuff>
                    //      #endif
                    //  #endif
                    //
                    // We don't want to actually add <stuff>.

                    if current.is_accepting_lines() || !parent_accepting {
                        current.stop_accepting_lines();
                    } else {
                        current.start_accepting_lines_if_appropriate();
                    }
                }
            } else if self.endif_regex.is_match(line) {
                scopes.pop();
                if scopes.is_empty() {
                    return Err(ProcessShaderError::TooManyEndIfs);
                }
            } else if scopes.last().unwrap().is_accepting_lines() {
                if let Some(cap) = SHADER_IMPORT_PROCESSOR
                    .import_asset_path_regex
                    .captures(line)
                {
                    let import = ShaderImport::AssetPath(cap.get(1).unwrap().as_str().to_string());
                    self.apply_import(
                        import_handles,
                        shaders,
                        &import,
                        shader,
                        shader_defs_unique,
                        &mut final_string,
                    )?;
                } else if let Some(cap) = SHADER_IMPORT_PROCESSOR
                    .import_custom_path_regex
                    .captures(line)
                {
                    let import = ShaderImport::Custom(cap.get(1).unwrap().as_str().to_string());
                    self.apply_import(
                        import_handles,
                        shaders,
                        &import,
                        shader,
                        shader_defs_unique,
                        &mut final_string,
                    )?;
                } else if SHADER_IMPORT_PROCESSOR
                    .define_import_path_regex
                    .is_match(line)
                {
                    // ignore import path lines
                } else if let Some(cap) = self.define_regex.captures(line) {
                    let def = cap.get(1).unwrap();
                    let name = def.as_str().to_string();

                    if let Some(val) = cap.get(2) {
                        if let Ok(val) = val.as_str().parse::<u32>() {
                            shader_defs_unique.insert(name.clone(), ShaderDefVal::UInt(name, val));
                        } else if let Ok(val) = val.as_str().parse::<i32>() {
                            shader_defs_unique.insert(name.clone(), ShaderDefVal::Int(name, val));
                        } else if let Ok(val) = val.as_str().parse::<bool>() {
                            shader_defs_unique.insert(name.clone(), ShaderDefVal::Bool(name, val));
                        } else {
                            return Err(ProcessShaderError::InvalidShaderDefDefinitionValue {
                                shader_def_name: name,
                                value: val.as_str().to_string(),
                            });
                        }
                    } else {
                        shader_defs_unique.insert(name.clone(), ShaderDefVal::Bool(name, true));
                    }
                } else {
                    let mut line_with_defs = line.to_string();
                    for capture in self.def_regex.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs_unique.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    for capture in self.def_regex_delimited.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs_unique.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex_delimited
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    final_string.push_str(&line_with_defs);
                    final_string.push('\n');
                }
            }
        }

        if scopes.len() != 1 {
            return Err(ProcessShaderError::NotEnoughEndIfs);
        }

        let processed_source = Cow::from(final_string);

        match &shader.source {
            Source::Wgsl(_source) => Ok(ProcessedShader::Wgsl(processed_source)),
            Source::Glsl(_source, stage) => Ok(ProcessedShader::Glsl(processed_source, *stage)),
            Source::SpirV(_source) => {
                unreachable!("SpirV has early return");
            }
        }
    }

    fn apply_import(
        &self,
        import_handles: &HashMap<ShaderImport, Handle<Shader>>,
        shaders: &HashMap<Handle<Shader>, Shader>,
        import: &ShaderImport,
        shader: &Shader,
        shader_defs_unique: &mut HashMap<String, ShaderDefVal>,
        final_string: &mut String,
    ) -> Result<(), ProcessShaderError> {
        let imported_shader = import_handles
            .get(import)
            .and_then(|handle| shaders.get(handle))
            .ok_or_else(|| ProcessShaderError::UnresolvedImport(import.clone()))?;
        let imported_processed =
            self.process_inner(imported_shader, shader_defs_unique, shaders, import_handles)?;

        match &shader.source {
            Source::Wgsl(_) => {
                if let ProcessedShader::Wgsl(import_source) = &imported_processed {
                    final_string.push_str(import_source);
                } else {
                    return Err(ProcessShaderError::MismatchedImportFormat(import.clone()));
                }
            }
            Source::Glsl(_, _) => {
                if let ProcessedShader::Glsl(import_source, _) = &imported_processed {
                    final_string.push_str(import_source);
                } else {
                    return Err(ProcessShaderError::MismatchedImportFormat(import.clone()));
                }
            }
            Source::SpirV(_) => {
                return Err(ProcessShaderError::ShaderFormatDoesNotSupportImports);
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use bevy_asset::{Handle, HandleUntyped};
    use bevy_reflect::TypeUuid;
    use bevy_utils::HashMap;
    use naga::ShaderStage;

    use crate::render_resource::{
        ProcessShaderError, Shader, ShaderDefVal, ShaderImport, ShaderProcessor,
    };
    #[rustfmt::skip]
const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_ELSE: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else
@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_ELSE_IFDEF: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef SECOND_TEXTURE
// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef THIRD_TEXTURE
// Third texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else
@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_ELSE_IFDEF_NO_ELSE_FALLBACK: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#ifdef TEXTURE
// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else ifdef OTHER_TEXTURE
// Other texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_NESTED_IFDEF: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

# ifdef TEXTURE
# ifdef ATTRIBUTE
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
# endif
# endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_NESTED_IFDEF_ELSE: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

# ifdef TEXTURE
# ifdef ATTRIBUTE
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#else
@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;
# endif
# endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    #[test]
    fn process_shader_def_defined() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &["TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_not_defined() {
        #[rustfmt::skip]
        const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_else() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_no_match_and_no_fallback_else() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF_NO_ELSE_FALLBACK),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_first_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

// Main texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF),
                &["TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_second_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF),
                &["SECOND_TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_ends_up_in_third_clause() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

// Third texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF),
                &["THIRD_TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_only_accepts_one_valid_else_ifdef() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

// Second texture
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_ELSE_IFDEF),
                &["SECOND_TEXTURE".into(), "THIRD_TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_else_ifdef_complicated_nesting() {
        // Test some nesting including #else ifdef statements
        // 1. Enter an #else ifdef
        // 2. Then enter an #else
        // 3. Then enter another #else ifdef

        #[rustfmt::skip]
        const WGSL_COMPLICATED_ELSE_IFDEF: &str = r"
#ifdef NOT_DEFINED
// not defined
#else ifdef IS_DEFINED
// defined 1
#ifdef NOT_DEFINED
// not defined
#else
// should be here
#ifdef NOT_DEFINED
// not defined
#else ifdef ALSO_NOT_DEFINED
// not defined
#else ifdef IS_DEFINED
// defined 2
#endif
#endif
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
// defined 1
// should be here
// defined 2
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_COMPLICATED_ELSE_IFDEF),
                &["IS_DEFINED".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_unclosed() {
        #[rustfmt::skip]
        const INPUT: &str = r"
#ifdef FOO
";
        let processor = ShaderProcessor::default();
        let result = processor.process(
            &Shader::from_wgsl(INPUT),
            &[],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(result, Err(ProcessShaderError::NotEnoughEndIfs));
    }

    #[test]
    fn process_shader_def_too_closed() {
        #[rustfmt::skip]
        const INPUT: &str = r"
#endif
";
        let processor = ShaderProcessor::default();
        let result = processor.process(
            &Shader::from_wgsl(INPUT),
            &[],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(result, Err(ProcessShaderError::TooManyEndIfs));
    }

    #[test]
    fn process_shader_def_commented() {
        #[rustfmt::skip]
        const INPUT: &str = r"
// #ifdef FOO
fn foo() { }
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(INPUT),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), INPUT);
    }

    #[test]
    fn process_import_wgsl() {
        #[rustfmt::skip]
        const FOO: &str = r"
fn foo() { }
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#import FOO
fn bar() { }
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"

fn foo() { }
fn bar() { }
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        let foo_handle = Handle::<Shader>::default();
        shaders.insert(foo_handle.clone_weak(), Shader::from_wgsl(FOO));
        import_handles.insert(
            ShaderImport::Custom("FOO".to_string()),
            foo_handle.clone_weak(),
        );
        let result = processor
            .process(&Shader::from_wgsl(INPUT), &[], &shaders, &import_handles)
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_import_glsl() {
        #[rustfmt::skip]
        const FOO: &str = r"
void foo() { }
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#import FOO
void bar() { }
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"

void foo() { }
void bar() { }
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        let foo_handle = Handle::<Shader>::default();
        shaders.insert(
            foo_handle.clone_weak(),
            Shader::from_glsl(FOO, ShaderStage::Vertex),
        );
        import_handles.insert(
            ShaderImport::Custom("FOO".to_string()),
            foo_handle.clone_weak(),
        );
        let result = processor
            .process(
                &Shader::from_glsl(INPUT, ShaderStage::Vertex),
                &[],
                &shaders,
                &import_handles,
            )
            .unwrap();
        assert_eq!(result.get_glsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_outer_defined_inner_not() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF),
                &["TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_outer_defined_inner_else() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d_array<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF_ELSE),
                &["TEXTURE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_neither_defined() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_neither_defined_else() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF_ELSE),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_inner_defined_outer_not() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF),
                &["ATTRIBUTE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_nested_shader_def_both_defined() {
        #[rustfmt::skip]
    const EXPECTED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL_NESTED_IFDEF),
                &["TEXTURE".into(), "ATTRIBUTE".into()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_import_ifdef() {
        #[rustfmt::skip]
        const FOO: &str = r"
#ifdef IMPORT_MISSING
fn in_import_missing() { }
#endif
#ifdef IMPORT_PRESENT
fn in_import_present() { }
#endif
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#import FOO
#ifdef MAIN_MISSING
fn in_main_missing() { }
#endif
#ifdef MAIN_PRESENT
fn in_main_present() { }
#endif
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"

fn in_import_present() { }
fn in_main_present() { }
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        let foo_handle = Handle::<Shader>::default();
        shaders.insert(foo_handle.clone_weak(), Shader::from_wgsl(FOO));
        import_handles.insert(
            ShaderImport::Custom("FOO".to_string()),
            foo_handle.clone_weak(),
        );
        let result = processor
            .process(
                &Shader::from_wgsl(INPUT),
                &["MAIN_PRESENT".into(), "IMPORT_PRESENT".into()],
                &shaders,
                &import_handles,
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_import_in_import() {
        #[rustfmt::skip]
        const BAR: &str = r"
#ifdef DEEP
fn inner_import() { }
#endif
";
        const FOO: &str = r"
#import BAR
fn import() { }
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#import FOO
fn in_main() { }
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"


fn inner_import() { }
fn import() { }
fn in_main() { }
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        {
            let bar_handle = Handle::<Shader>::default();
            shaders.insert(bar_handle.clone_weak(), Shader::from_wgsl(BAR));
            import_handles.insert(
                ShaderImport::Custom("BAR".to_string()),
                bar_handle.clone_weak(),
            );
        }
        {
            let foo_handle = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1).typed();
            shaders.insert(foo_handle.clone_weak(), Shader::from_wgsl(FOO));
            import_handles.insert(
                ShaderImport::Custom("FOO".to_string()),
                foo_handle.clone_weak(),
            );
        }
        let result = processor
            .process(
                &Shader::from_wgsl(INPUT),
                &["DEEP".into()],
                &shaders,
                &import_handles,
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_import_in_ifdef() {
        #[rustfmt::skip]
        const BAR: &str = r"
fn bar() { }
";
        #[rustfmt::skip]
        const BAZ: &str = r"
fn baz() { }
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#ifdef FOO
    #import BAR
#else
    #import BAZ
#endif
";
        #[rustfmt::skip]
        const EXPECTED_FOO: &str = r"

fn bar() { }
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"

fn baz() { }
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        {
            let bar_handle = Handle::<Shader>::default();
            shaders.insert(bar_handle.clone_weak(), Shader::from_wgsl(BAR));
            import_handles.insert(
                ShaderImport::Custom("BAR".to_string()),
                bar_handle.clone_weak(),
            );
        }
        {
            let baz_handle = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1).typed();
            shaders.insert(baz_handle.clone_weak(), Shader::from_wgsl(BAZ));
            import_handles.insert(
                ShaderImport::Custom("BAZ".to_string()),
                baz_handle.clone_weak(),
            );
        }
        let result = processor
            .process(
                &Shader::from_wgsl(INPUT),
                &["FOO".into()],
                &shaders,
                &import_handles,
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED_FOO);

        let result = processor
            .process(&Shader::from_wgsl(INPUT), &[], &shaders, &import_handles)
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_def_unknown_operator() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#if TEXTURE !! true
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        let processor = ShaderProcessor::default();

        let result_missing = processor.process(
            &Shader::from_wgsl(WGSL),
            &["TEXTURE".into()],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(
            result_missing,
            Err(ProcessShaderError::UnknownShaderDefOperator {
                operator: "!!".to_string()
            })
        );
    }
    #[test]
    fn process_shader_def_equal_int() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#if TEXTURE == 3
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result_eq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Int("TEXTURE".to_string(), 3)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_eq.get_wgsl_source().unwrap(), EXPECTED_EQ);

        let result_neq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Int("TEXTURE".to_string(), 7)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_neq.get_wgsl_source().unwrap(), EXPECTED_NEQ);

        let result_missing = processor.process(
            &Shader::from_wgsl(WGSL),
            &[],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(
            result_missing,
            Err(ProcessShaderError::UnknownShaderDef {
                shader_def_name: "TEXTURE".to_string()
            })
        );

        let result_wrong_type = processor.process(
            &Shader::from_wgsl(WGSL),
            &[ShaderDefVal::Bool("TEXTURE".to_string(), true)],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(
            result_wrong_type,
            Err(ProcessShaderError::InvalidShaderDefComparisonValue {
                shader_def_name: "TEXTURE".to_string(),
                expected: "bool".to_string(),
                value: "3".to_string()
            })
        );
    }

    #[test]
    fn process_shader_def_equal_bool() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#if TEXTURE == true
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result_eq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Bool("TEXTURE".to_string(), true)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_eq.get_wgsl_source().unwrap(), EXPECTED_EQ);

        let result_neq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Bool("TEXTURE".to_string(), false)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_neq.get_wgsl_source().unwrap(), EXPECTED_NEQ);
    }

    #[test]
    fn process_shader_def_not_equal_bool() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

#if TEXTURE != false
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_EQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_NEQ: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;


struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result_eq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Bool("TEXTURE".to_string(), true)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_eq.get_wgsl_source().unwrap(), EXPECTED_EQ);

        let result_neq = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[ShaderDefVal::Bool("TEXTURE".to_string(), false)],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result_neq.get_wgsl_source().unwrap(), EXPECTED_NEQ);

        let result_missing = processor.process(
            &Shader::from_wgsl(WGSL),
            &[],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(
            result_missing,
            Err(ProcessShaderError::UnknownShaderDef {
                shader_def_name: "TEXTURE".to_string()
            })
        );

        let result_wrong_type = processor.process(
            &Shader::from_wgsl(WGSL),
            &[ShaderDefVal::Int("TEXTURE".to_string(), 7)],
            &HashMap::default(),
            &HashMap::default(),
        );
        assert_eq!(
            result_wrong_type,
            Err(ProcessShaderError::InvalidShaderDefComparisonValue {
                shader_def_name: "TEXTURE".to_string(),
                expected: "int".to_string(),
                value: "false".to_string()
            })
        );
    }

    #[test]
    fn process_shader_def_replace() {
        #[rustfmt::skip]
        const WGSL: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    var a: i32 = #FIRST_VALUE;
    var b: i32 = #FIRST_VALUE * #SECOND_VALUE;
    var c: i32 = #MISSING_VALUE;
    var d: bool = #BOOL_VALUE;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

        #[rustfmt::skip]
        const EXPECTED_REPLACED: &str = r"
struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    var a: i32 = 5;
    var b: i32 = 5 * 3;
    var c: i32 = #MISSING_VALUE;
    var d: bool = true;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[
                    ShaderDefVal::Bool("BOOL_VALUE".to_string(), true),
                    ShaderDefVal::Int("FIRST_VALUE".to_string(), 5),
                    ShaderDefVal::Int("SECOND_VALUE".to_string(), 3),
                ],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED_REPLACED);
    }

    #[test]
    fn process_shader_define_in_shader() {
        #[rustfmt::skip]
        const WGSL: &str = r"
#ifdef NOW_DEFINED
defined at start
#endif
#define NOW_DEFINED
#ifdef NOW_DEFINED
defined at end
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
defined at end
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_define_only_in_accepting_scopes() {
        #[rustfmt::skip]
        const WGSL: &str = r"
#define GUARD
#ifndef GUARD
#define GUARDED
#endif
#ifdef GUARDED
This should not be part of the result
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_define_in_shader_with_value() {
        #[rustfmt::skip]
        const WGSL: &str = r"
#define DEFUINT 1
#define DEFINT -1
#define DEFBOOL false
#if DEFUINT == 1
uint: #DEFUINT
#endif
#if DEFINT == -1
int: #DEFINT
#endif
#if DEFBOOL == false
bool: #DEFBOOL
#endif
";

        #[rustfmt::skip]
        const EXPECTED: &str = r"
uint: 1
int: -1
bool: false
";
        let processor = ShaderProcessor::default();
        let result = processor
            .process(
                &Shader::from_wgsl(WGSL),
                &[],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }

    #[test]
    fn process_shader_define_across_imports() {
        #[rustfmt::skip]
        const FOO: &str = r"
#define IMPORTED
";
        const BAR: &str = r"
#IMPORTED
";
        #[rustfmt::skip]
        const INPUT: &str = r"
#import FOO
#import BAR
";
        #[rustfmt::skip]
        const EXPECTED: &str = r"


true
";
        let processor = ShaderProcessor::default();
        let mut shaders = HashMap::default();
        let mut import_handles = HashMap::default();
        {
            let foo_handle = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0).typed();
            shaders.insert(foo_handle.clone_weak(), Shader::from_wgsl(FOO));
            import_handles.insert(
                ShaderImport::Custom("FOO".to_string()),
                foo_handle.clone_weak(),
            );
        }
        {
            let bar_handle = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1).typed();
            shaders.insert(bar_handle.clone_weak(), Shader::from_wgsl(BAR));
            import_handles.insert(
                ShaderImport::Custom("BAR".to_string()),
                bar_handle.clone_weak(),
            );
        }
        let result = processor
            .process(&Shader::from_wgsl(INPUT), &[], &shaders, &import_handles)
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }
}
