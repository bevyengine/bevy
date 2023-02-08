use super::ShaderDefVal;
use crate::define_atomic_id;
use bevy_asset::{AssetLoader, AssetPath, Handle, LoadContext, LoadedAsset};
use bevy_reflect::TypeUuid;
use bevy_utils::{tracing::error, BoxedFuture};

use regex::Regex;
use std::{borrow::Cow, marker::Copy};
use thiserror::Error;

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
/// A shader, as defined by its [`ShaderSource`](wgpu::ShaderSource) and [`ShaderStage`](naga::ShaderStage)
/// This is an "unprocessed" shader. It can contain preprocessor directives.
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    pub path: String,
    pub source: Source,
    pub import_path: ShaderImport,
    pub imports: Vec<ShaderImport>,
    // extra imports not specified in the source string
    pub additional_imports: Vec<naga_oil::compose::ImportDefinition>,
    // any shader defs that will be included when this module is used
    pub shader_defs: Vec<ShaderDefVal>,
}

impl Shader {
    fn preprocess(source: &str, path: &str) -> (ShaderImport, Vec<ShaderImport>) {
        let (import_path, imports) = naga_oil::compose::get_preprocessor_data(source);

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
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Cow<'static, [u8]>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

impl Source {
    pub fn as_str(&self) -> &str {
        match self {
            Source::Wgsl(s) | Source::Glsl(s, _) => s,
            Source::SpirV(_) => panic!("spirv not yet implemented"),
        }
    }
}

impl From<&Source> for naga_oil::compose::ShaderLanguage {
    fn from(value: &Source) -> Self {
        match value {
            Source::Wgsl(_) => naga_oil::compose::ShaderLanguage::Wgsl,
            Source::Glsl(_, _) => naga_oil::compose::ShaderLanguage::Glsl,
            Source::SpirV(_) => panic!("spirv not yet implemented"),
        }
    }
}

impl From<&Source> for naga_oil::compose::ShaderType {
    fn from(value: &Source) -> Self {
        match value {
            Source::Wgsl(_) => naga_oil::compose::ShaderType::Wgsl,
            Source::Glsl(_, naga::ShaderStage::Vertex) => naga_oil::compose::ShaderType::GlslVertex,
            Source::Glsl(_, naga::ShaderStage::Fragment) => {
                naga_oil::compose::ShaderType::GlslFragment
            }
            Source::Glsl(_, naga::ShaderStage::Compute) => {
                panic!("glsl compute not yet implemented")
            }
            Source::SpirV(_) => panic!("spirv not yet implemented"),
        }
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

            let shader = match ext {
                "spv" => {
                    Shader::from_spirv(Vec::from(bytes), load_context.path().to_string_lossy())
                }
                "wgsl" => Shader::from_wgsl(
                    String::from_utf8(Vec::from(bytes))?,
                    load_context.path().to_string_lossy(),
                ),
                "vert" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Vertex,
                    load_context.path().to_string_lossy(),
                ),
                "frag" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Fragment,
                    load_context.path().to_string_lossy(),
                ),
                "comp" => Shader::from_glsl(
                    String::from_utf8(Vec::from(bytes))?,
                    naga::ShaderStage::Compute,
                    load_context.path().to_string_lossy(),
                ),
                _ => panic!("unhandled extension: {ext}"),
            };

            // collect file dependencies
            let dependencies = shader
                .imports
                .iter()
                .flat_map(|import| {
                    if let ShaderImport::AssetPath(asset_path) = import {
                        Some(asset_path.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let mut asset = LoadedAsset::new(shader);
            for dependency in dependencies {
                asset.add_dependency(dependency.into());
            }

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["spv", "wgsl", "vert", "frag", "comp"]
    }
}

pub struct ShaderImportProcessor {
    import_asset_path_regex: Regex,
    import_custom_path_regex: Regex,
    import_items_regex: Regex,
    define_import_path_regex: Regex,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ShaderImport {
    AssetPath(String),
    Custom(String),
}

impl ShaderImport {
    pub fn as_str(&self) -> &str {
        match self {
            ShaderImport::AssetPath(s) | ShaderImport::Custom(s) => s,
        }
    }
}

impl Default for ShaderImportProcessor {
    fn default() -> Self {
        Self {
            import_asset_path_regex: Regex::new(r#"^\s*#\s*import\s+"([^\s]+)""#).unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)").unwrap(),
            import_items_regex: Regex::new(r"^\s*#\s*from\s+([^\s]+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+([^\s]+)").unwrap(),
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
            } else if let Some(cap) = self.import_items_regex.captures(line) {
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
}
