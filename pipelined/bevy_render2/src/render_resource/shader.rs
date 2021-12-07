use bevy_asset::{AssetLoader, Handle, LoadContext, LoadedAsset};
use bevy_reflect::{TypeUuid, Uuid};
use bevy_utils::{tracing::error, BoxedFuture, HashMap};
use naga::{valid::ModuleInfo, Module};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    borrow::Cow, collections::HashSet, marker::Copy, ops::Deref, path::PathBuf, str::FromStr,
};
use thiserror::Error;
use wgpu::{ShaderModuleDescriptor, ShaderSource};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ShaderId(Uuid);

impl ShaderId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ShaderId(Uuid::new_v4())
    }
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error(transparent)]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[error("GLSL Parse Error: {0:?}")]
    GlslParse(Vec<naga::front::glsl::Error>),
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::valid::ValidationError),
}

/// A shader, as defined by its [ShaderSource] and [ShaderStage]
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
        Shader {
            imports: SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source),
            source: Source::Wgsl(source),
            import_path: None,
        }
    }

    pub fn from_glsl(source: impl Into<Cow<'static, str>>, stage: naga::ShaderStage) -> Shader {
        let source = source.into();
        Shader {
            imports: SHADER_IMPORT_PROCESSOR.get_imports_from_str(&source),
            source: Source::Glsl(source, stage),
            import_path: None,
        }
    }

    pub fn from_spirv(source: impl Into<Cow<'static, [u8]>>) -> Shader {
        Shader {
            imports: Vec::new(),
            source: Source::SpirV(source.into()),
            import_path: None,
        }
    }

    pub fn set_import_path<P: Into<String>>(&mut self, import_path: P) {
        self.import_path = Some(ShaderImport::Custom(import_path.into()));
    }

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

    pub fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError> {
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
        let module_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::default(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)?;

        Ok(ShaderReflection {
            module,
            module_info,
        })
    }

    pub fn get_module_descriptor(&self) -> Result<ShaderModuleDescriptor, AsModuleDescriptorError> {
        Ok(ShaderModuleDescriptor {
            label: None,
            source: match self {
                ProcessedShader::Wgsl(source) => ShaderSource::Wgsl(source.clone()),
                ProcessedShader::Glsl(_source, _stage) => {
                    let reflection = self.reflect()?;
                    // TODO: it probably makes more sense to convert this to spirv, but as of writing
                    // this comment, naga's spirv conversion is broken
                    let wgsl = reflection.get_wgsl()?;
                    ShaderSource::Wgsl(wgsl.into())
                }
                ProcessedShader::SpirV(_) => {
                    // TODO: we can probably just transmute the u8 array to u32?
                    let reflection = self.reflect()?;
                    let spirv = reflection.get_spirv()?;
                    ShaderSource::SpirV(Cow::Owned(spirv))
                }
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
        naga::back::wgsl::write_string(&self.module, &self.module_info)
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
                _ => panic!("unhandled extension: {}", ext),
            };

            shader.import_path = Some(ShaderImport::AssetPath(
                load_context.path().to_string_lossy().to_string(),
            ));
            let imports = SHADER_IMPORT_PROCESSOR.get_imports(&shader);
            let mut asset = LoadedAsset::new(shader);
            for import in imports {
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
        &["spv", "wgsl", "vert", "frag"]
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
    #[error("This Shader's format does not support processing shader defs.")]
    ShaderFormatDoesNotSupportShaderDefs,
    #[error("This Shader's formatdoes not support imports.")]
    ShaderFormatDoesNotSupportImports,
    #[error("Unresolved import: {0:?}.")]
    UnresolvedImport(ShaderImport),
    #[error("The shader import {0:?} does not match the source file type. Support for this might be added in the future.")]
    MismatchedImportFormat(ShaderImport),
}

pub struct ShaderImportProcessor {
    import_asset_path_regex: Regex,
    import_custom_path_regex: Regex,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    AssetPath(String),
    Custom(String),
}

impl Default for ShaderImportProcessor {
    fn default() -> Self {
        Self {
            import_asset_path_regex: Regex::new(r#"^\s*#\s*import\s*"(.+)""#).unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s*(.+)").unwrap(),
        }
    }
}

impl ShaderImportProcessor {
    pub fn get_imports(&self, shader: &Shader) -> Vec<ShaderImport> {
        match &shader.source {
            Source::Wgsl(source) => self.get_imports_from_str(source),
            Source::Glsl(source, _stage) => self.get_imports_from_str(source),
            Source::SpirV(_source) => Vec::new(),
        }
    }

    pub fn get_imports_from_str(&self, shader: &str) -> Vec<ShaderImport> {
        let mut imports = Vec::new();
        for line in shader.split('\n') {
            if let Some(cap) = self.import_asset_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                imports.push(ShaderImport::AssetPath(import.as_str().to_string()));
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                let import = cap.get(1).unwrap();
                imports.push(ShaderImport::Custom(import.as_str().to_string()));
            }
        }

        imports
    }
}

pub static SHADER_IMPORT_PROCESSOR: Lazy<ShaderImportProcessor> =
    Lazy::new(ShaderImportProcessor::default);

pub struct ShaderProcessor {
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
}

impl Default for ShaderProcessor {
    fn default() -> Self {
        Self {
            ifdef_regex: Regex::new(r"^\s*#\s*ifdef\s*([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*ifndef\s*([\w|\d|_]+)").unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
        }
    }
}

impl ShaderProcessor {
    pub fn process(
        &self,
        shader: &Shader,
        shader_defs: &[String],
        shaders: &HashMap<Handle<Shader>, Shader>,
        import_handles: &HashMap<ShaderImport, Handle<Shader>>,
    ) -> Result<ProcessedShader, ProcessShaderError> {
        let shader_str = match &shader.source {
            Source::Wgsl(source) => source.deref(),
            Source::Glsl(source, _stage) => source.deref(),
            Source::SpirV(source) => {
                if shader_defs.is_empty() {
                    return Ok(ProcessedShader::SpirV(source.clone()));
                } else {
                    return Err(ProcessShaderError::ShaderFormatDoesNotSupportShaderDefs);
                }
            }
        };

        let shader_defs = HashSet::<String>::from_iter(shader_defs.iter().cloned());
        let mut scopes = vec![true];
        let mut final_string = String::new();
        for line in shader_str.split('\n') {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && shader_defs.contains(def.as_str()));
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && !shader_defs.contains(def.as_str()));
            } else if self.else_regex.is_match(line) {
                let mut is_parent_scope_truthy = true;
                if scopes.len() > 1 {
                    is_parent_scope_truthy = scopes[scopes.len() - 2];
                }
                if let Some(last) = scopes.last_mut() {
                    *last = is_parent_scope_truthy && !*last;
                }
            } else if self.endif_regex.is_match(line) {
                scopes.pop();
                if scopes.is_empty() {
                    return Err(ProcessShaderError::TooManyEndIfs);
                }
            } else if let Some(cap) = SHADER_IMPORT_PROCESSOR
                .import_asset_path_regex
                .captures(line)
            {
                let import = ShaderImport::AssetPath(cap.get(1).unwrap().as_str().to_string());
                apply_import(import_handles, shaders, &import, shader, &mut final_string)?;
            } else if let Some(cap) = SHADER_IMPORT_PROCESSOR
                .import_custom_path_regex
                .captures(line)
            {
                let import = ShaderImport::Custom(cap.get(1).unwrap().as_str().to_string());
                apply_import(import_handles, shaders, &import, shader, &mut final_string)?;
            } else if *scopes.last().unwrap() {
                final_string.push_str(line);
                final_string.push('\n');
            }
        }

        final_string.pop();

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
}

fn apply_import(
    import_handles: &HashMap<ShaderImport, Handle<Shader>>,
    shaders: &HashMap<Handle<Shader>, Shader>,
    import: &ShaderImport,
    shader: &Shader,
    final_string: &mut String,
) -> Result<(), ProcessShaderError> {
    let imported_shader = import_handles
        .get(import)
        .and_then(|handle| shaders.get(handle))
        .ok_or_else(|| ProcessShaderError::UnresolvedImport(import.clone()))?;
    match &shader.source {
        Source::Wgsl(_) => {
            if let Source::Wgsl(import_source) = &imported_shader.source {
                final_string.push_str(import_source);
            } else {
                return Err(ProcessShaderError::MismatchedImportFormat(import.clone()));
            }
        }
        Source::Glsl(_, _) => {
            if let Source::Glsl(import_source, _) = &imported_shader.source {
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

#[cfg(test)]
mod tests {
    use bevy_asset::Handle;
    use bevy_utils::HashMap;
    use naga::ShaderStage;

    use crate::render_resource::{ProcessShaderError, Shader, ShaderImport, ShaderProcessor};
    #[rustfmt::skip]
const WGSL: &str = r"
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

#ifdef TEXTURE
[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
#endif

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_ELSE: &str = r"
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

#ifdef TEXTURE
[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
#else
[[group(1), binding(0)]]
var sprite_texture: texture_2d_array<f32>;
#endif

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_NESTED_IFDEF: &str = r"
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

# ifdef TEXTURE
# ifdef ATTRIBUTE
[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
# endif
# endif

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    return out;
}
";

    const WGSL_NESTED_IFDEF_ELSE: &str = r"
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

# ifdef TEXTURE
# ifdef ATTRIBUTE
[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;
#else
[[group(1), binding(0)]]
var sprite_texture: texture_2d_array<f32>;
# endif
# endif

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
                &["TEXTURE".to_string()],
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;


struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var sprite_texture: texture_2d_array<f32>;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;


struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
                &["TEXTURE".to_string()],
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var sprite_texture: texture_2d_array<f32>;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
                &["TEXTURE".to_string()],
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;


struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;


struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;


struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
                &["ATTRIBUTE".to_string()],
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
[[block]]
struct View {
    view_proj: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var sprite_texture: texture_2d<f32>;

struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] vertex_position: vec3<f32>,
    [[location(1)]] vertex_uv: vec2<f32>
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
                &["TEXTURE".to_string(), "ATTRIBUTE".to_string()],
                &HashMap::default(),
                &HashMap::default(),
            )
            .unwrap();
        assert_eq!(result.get_wgsl_source().unwrap(), EXPECTED);
    }
}
