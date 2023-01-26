/// the compose module allows construction of shaders from modules (which are themselves shaders).
///
/// it does this by treating shaders as modules, and
/// - building each module independently to naga IR
/// - creating "header" files for each supported language, which are used to build dependent modules/shaders
/// - making final shaders by combining the shader IR with the IR for imported modules
///
/// for multiple small shaders with large common imports, this can be faster than parsing the full source for each shader, and it allows for constructing shaders in a cleaner modular manner with better scope control.
///
/// ## imports
///
/// shaders can be added to the composer as modules. this makes their types, constants, variables and functions available to modules/shaders that import them. note that importing a module will affect the final shader's global state if the module defines globals variables with bindings.
///
/// modules must include a `#define_import_path` directive that names the module.
///
/// ```ignore
/// #define_import_path my_module
///
/// fn my_func() -> f32 {
///     return 1.0;
/// }
/// ```
///
/// shaders can then import the module with an `#import` directive (with an optional `as` name). at point of use, imported items must be qualified:
///
/// ```ignore
/// #import my_module
/// #import my_other_module as Mod2
///
/// fn main() -> f32 {
///     let x = my_module::my_func();
///     let y = Mod2::my_other_func();
///     return x*y;
/// }
/// ```
///
/// or import a comma-separated list of individual items with a `#from` directive. at point of use, imported items must be prefixed with `::` :
///
/// ```ignore
/// #from my_module import my_func, my_const
///
/// fn main() -> f32 {
///     return ::my_func(::my_const);
/// }
/// ```
///
/// imports can be nested - modules may import other modules, but not recursively. when a new module is added, all its `#import`s must already have been added.
/// the same module can be imported multiple times by different modules in the import tree.
/// there is no overlap of namespaces, so the same function names (or type, constant, or variable names) may be used in different modules.
///
/// note: when importing an item with the `#from` directive, the final shader will include the required dependencies (bindings, globals, consts, other functions) of the imported item, but will not include the rest of the imported module. it will however still include all of any modules imported by the imported module. this is probably not desired in general and may be fixed in a future version. currently for a more complete culling of unused dependencies the `prune` module can be used.
///
/// ## overriding functions
///
/// virtual functions can be declared with the `virtual` keyword:
/// ```ignore
/// virtual fn point_light(world_position: vec3<f32>) -> vec3<f32> { ... }
/// ```
/// virtual functions defined in imported modules can then be overridden using the `override` keyword:
///
/// ```ignore
/// #import bevy_pbr::lighting as Lighting
///
/// override fn Lighting::point_light (world_position: vec3<f32>) -> vec3<f32> {
///     let original = Lighting::point_light(world_position);
///     let quantized = vec3<u32>(original * 3.0);
///     return vec3<f32>(quantized) / 3.0;
/// }
/// ```
///
/// override function definitions cause *all* calls to the original function in the entire shader scope to be replaced by calls to the new function, with the exception of calls within the override function itself.
///
/// the function signature of the override must match the base function.
///
/// overrides can be specified at any point in the final shader's import tree.
///
/// multiple overrides can be applied to the same function. for example, given :
/// - a module `a` containing a function `f`,
/// - a module `b` that imports `a`, and containing an `override a::f` function,
/// - a module `c` that imports `a` and `b`, and containing an `override a::f` function,
/// then b and c both specify an override for `a::f`.
/// the `override fn a::f` declared in module `b` may call to `a::f` within its body.
/// the `override fn a::f` declared in module 'c' may call to `a::f` within its body, but the call will be redirected to `b::f`.
/// any other calls to `a::f` (within modules 'a' or `b`, or anywhere else) will end up redirected to `c::f`
/// in this way a chain or stack of overrides can be applied.
///
/// different overrides of the same function can be specified in different import branches. the final stack will be ordered based on the first occurrence of the override in the import tree (using a depth first search).
///
/// note that imports into a module/shader are processed in order, but are processed before the body of the current shader/module regardless of where they occur in that module, so there is no way to import a module containing an override and inject a call into the override stack prior to that imported override. you can instead create two modules each containing an override and import them into a parent module/shader to order them as required.
/// override functions can currently only be defined in wgsl.
///
/// if the `override_any` crate feature is enabled, then the `virtual` keyword is not required for the function being overridden.
///
/// ## languages
///
/// modules can we written in GLSL or WGSL. shaders with entry points can be imported as modules (provided they have a `#define_import_path` directive). entry points are available to call from imported modules either via their name (for WGSL) or via `module::main` (for GLSL).
///
/// final shaders can also be written in GLSL or WGSL. for GLSL users must specify whether the shader is a vertex shader or fragment shader via the `ShaderType` argument (GLSL compute shaders are not supported).
///
/// ## preprocessing
///
/// when generating a final shader or adding a composable module, a set of `shader_def` string/value pairs must be provided. The value can be a bool (`ShaderDefValue::Bool`), an i32 (`ShaderDefValue::Int`) or a u32 (`ShaderDefValue::UInt`).
///
/// these allow conditional compilation of parts of modules and the final shader. conditional compilation is performed with `#if` / `#ifdef` / `#ifndef`, `#else` and `#endif` preprocessor directives:
///
/// ```ignore
/// fn get_number() -> f32 {
///     #ifdef BIG_NUMBER
///         return 999.0;
///     #else
///         return 0.999;
///     #endif
/// }
/// ```
/// the `#ifdef` directive matches when the def name exists in the input binding set (regardless of value). the `#ifndef` directive is the reverse.
///
/// the `#if` directive requires a def name, an operator, and a value for comparison:
/// - the def name must be a provided `shader_def` name.
/// - the operator must be one of `==`, `!=`, `>=`, `>`, `<`, `<=`
/// - the value must be an integer literal if comparing to a `ShaderDef::Int`, or `true` or `false` if comparing to a `ShaderDef::Bool`.
///
/// shader defs can also be used in the shader source with `#SHADER_DEF` or `#{SHADER_DEF}`, and will be substituted for their value.
///
/// ## error reporting
///
/// codespan reporting for errors is available using the error `emit_to_string` method. this requires validation to be enabled, which is true by default. `Composer::non_validating()` produces a non-validating composer that is not able to give accurate error reporting.
///
use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term,
};
use naga::EntryPoint;
use regex::Regex;
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    iter::FusedIterator,
    ops::Range,
};
use thiserror::Error;
use tracing::{debug, trace, warn};

use crate::{
    derive::DerivedModule,
    redirect::{RedirectError, Redirector},
};

mod test;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum ShaderLanguage {
    #[default]
    Wgsl,
    Glsl,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum ShaderType {
    #[default]
    Wgsl,
    GlslVertex,
    GlslFragment,
}

impl From<ShaderType> for ShaderLanguage {
    fn from(ty: ShaderType) -> Self {
        match ty {
            ShaderType::Wgsl => ShaderLanguage::Wgsl,
            ShaderType::GlslVertex | ShaderType::GlslFragment => ShaderLanguage::Glsl,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShaderDefValue {
    Bool(bool),
    Int(i32),
    UInt(u32),
}

impl Default for ShaderDefValue {
    fn default() -> Self {
        ShaderDefValue::Bool(true)
    }
}

impl ShaderDefValue {
    fn value_as_string(&self) -> String {
        match self {
            ShaderDefValue::Bool(val) => val.to_string(),
            ShaderDefValue::Int(val) => val.to_string(),
            ShaderDefValue::UInt(val) => val.to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct OwnedShaderDefs(BTreeMap<String, ShaderDefValue>);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct ModuleKey(OwnedShaderDefs);

impl ModuleKey {
    fn from_members(key: &HashMap<String, ShaderDefValue>, universe: &[String]) -> Self {
        let mut acc = OwnedShaderDefs::default();
        for item in universe {
            if let Some(value) = key.get(item) {
                acc.0.insert(item.to_owned(), *value);
            }
        }
        ModuleKey(acc)
    }
}

// a module built with a specific set of shader_defs
#[derive(Default, Debug)]
pub struct ComposableModule {
    // module decoration, prefixed to all items from this module in the final source
    pub decorated_name: String,
    // module names required as imports, optionally with a list of items to import
    pub imports: Vec<ImportDefinition>,
    // types exported
    pub owned_types: HashSet<String>,
    // constants exported
    pub owned_constants: HashSet<String>,
    // vars exported
    pub owned_vars: HashSet<String>,
    // functions exported
    pub owned_functions: HashSet<String>,
    // local functions that can be overridden
    pub virtual_functions: HashSet<String>,
    // overriding functions defined in this module
    // target function -> Vec<replacement functions>
    pub override_functions: HashMap<String, Vec<String>>,
    // naga module, built against headers for any imports
    module_ir: naga::Module,
    // headers in different shader languages, used for building modules/shaders that import this module
    // headers contain types, constants, global vars and empty function definitions -
    // just enough to convert source strings that want to import this module into naga IR
    headers: HashMap<ShaderLanguage, String>,
    // character offset of the start of the owned module string
    start_offset: usize,
}

// data used to build a ComposableModule
#[derive(Debug)]
pub struct ComposableModuleDefinition {
    pub name: String,
    // shader text
    pub substituted_source: String,
    // language
    pub language: ShaderLanguage,
    // source path for error display
    pub file_path: String,
    // shader def values bound to this module
    pub shader_defs: HashMap<String, ShaderDefValue>,
    // list of shader_defs that can affect this module
    effective_defs: Vec<String>,
    // full list of possible imports (regardless of shader_def configuration)
    all_imports: HashSet<String>,
    // additional imports to add (as though they were included in the source after any other imports)
    additional_imports: Vec<ImportDefinition>,
    // built composable modules for a given set of shader defs
    modules: HashMap<ModuleKey, ComposableModule>,
    // used in spans when this module is included
    module_index: usize,
}

impl ComposableModuleDefinition {
    fn get_module(
        &self,
        shader_defs: &HashMap<String, ShaderDefValue>,
    ) -> Option<&ComposableModule> {
        self.modules
            .get(&ModuleKey::from_members(shader_defs, &self.effective_defs))
    }

    fn insert_module(
        &mut self,
        shader_defs: &HashMap<String, ShaderDefValue>,
        module: ComposableModule,
    ) -> &ComposableModule {
        match self
            .modules
            .entry(ModuleKey::from_members(shader_defs, &self.effective_defs))
        {
            Entry::Occupied(_) => panic!("entry already populated"),
            Entry::Vacant(v) => v.insert(module),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportDefinition {
    pub import: String,
    pub as_name: Option<String>,
    pub items: Option<Vec<String>>,
}

impl ImportDefinition {
    fn as_name(&self) -> &str {
        self.as_name.as_deref().unwrap_or(self.import.as_str())
    }
}

#[derive(Debug, Clone)]
struct ImportDefWithOffset {
    definition: ImportDefinition,
    offset: usize,
}

#[derive(Debug)]
enum ErrSource {
    Module(String, usize),
    Constructing {
        path: String,
        source: String,
        offset: usize,
    },
}

impl ErrSource {
    fn path<'a>(&'a self, composer: &'a Composer) -> &'a String {
        match self {
            ErrSource::Module(c, _) => &composer.module_sets.get(c).unwrap().file_path,
            ErrSource::Constructing { path, .. } => path,
        }
    }

    fn source<'a>(&'a self, composer: &'a Composer) -> &'a String {
        match self {
            ErrSource::Module(c, _) => &composer.module_sets.get(c).unwrap().substituted_source,
            ErrSource::Constructing { source, .. } => source,
        }
    }

    fn offset(&self) -> usize {
        match self {
            ErrSource::Module(_, offset) | ErrSource::Constructing { offset, .. } => *offset,
        }
    }
}

#[derive(Debug, Error)]
#[error("Composer error: {inner}")]
pub struct ComposerError {
    #[source]
    inner: ComposerErrorInner,
    source: ErrSource,
}

#[derive(Debug, Error)]
pub enum ComposerErrorInner {
    #[error("required import '{0}' not found")]
    ImportNotFound(String, usize),
    #[error("{0}")]
    WgslParseError(naga::front::wgsl::ParseError),
    #[error("{0:?}")]
    GlslParseError(Vec<naga::front::glsl::Error>),
    #[error("naga_oil bug, please file a report: failed to convert imported module IR back into WGSL for use with WGSL shaders: {0}")]
    WgslBackError(naga::back::wgsl::Error),
    #[error("naga_oil bug, please file a report: failed to convert imported module IR back into GLSL for use with GLSL shaders: {0}")]
    GlslBackError(naga::back::glsl::Error),
    #[error("naga_oil bug, please file a report: composer failed to build a valid header: {0}")]
    HeaderValidationError(naga::WithSpan<naga::valid::ValidationError>),
    #[error("failed to build a valid final module: {0}")]
    ShaderValidationError(naga::WithSpan<naga::valid::ValidationError>),
    #[error(
        "Not enough '# endif' lines. Each if statement should be followed by an endif statement."
    )]
    NotEnoughEndIfs(usize),
    #[error("Too many '# endif' lines. Each endif should be preceded by an if statement.")]
    TooManyEndIfs(usize),
    #[error("Unknown shader def operator: '{operator}'")]
    UnknownShaderDefOperator { pos: usize, operator: String },
    #[error("Unknown shader def: '{shader_def_name}'")]
    UnknownShaderDef { pos: usize, shader_def_name: String },
    #[error(
        "Invalid shader def comparison for '{shader_def_name}': expected {expected}, got {value}"
    )]
    InvalidShaderDefComparisonValue {
        pos: usize,
        shader_def_name: String,
        expected: String,
        value: String,
    },
    #[error("multiple inconsistent shader def values: '{def}'")]
    InconsistentShaderDefValue { def: String },
    #[error("Attempted to add a module with no #define_import_path")]
    NoModuleName,
    #[error("source contains internal decoration string, results probably won't be what you expect. if you have a legitimate reason to do this please file a report")]
    DecorationInSource(Range<usize>),
    #[error("naga oil only supports glsl 440 and 450")]
    GlslInvalidVersion(usize),
    #[error("invalid override :{0}")]
    RedirectError(#[from] RedirectError),
    #[error(
        "override is invalid as `{name}` is not virtual (this error can be disabled with feature 'override_any')"
    )]
    OverrideNotVirtual { name: String, pos: usize },
    #[error(
        "Composable module identifiers must not require substitution according to naga writeback rules: `{original}`"
    )]
    InvalidIdentifier { original: String, at: naga::Span },
}

struct ErrorSources<'a> {
    current: Option<&'a (dyn std::error::Error + 'static)>,
}

impl<'a> ErrorSources<'a> {
    fn of(error: &'a dyn std::error::Error) -> Self {
        Self {
            current: error.source(),
        }
    }
}

impl<'a> Iterator for ErrorSources<'a> {
    type Item = &'a (dyn std::error::Error + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = self.current.and_then(std::error::Error::source);
        current
    }
}

impl<'a> FusedIterator for ErrorSources<'a> {}

impl ComposerError {
    /// format a Composer error
    pub fn emit_to_string(&self, composer: &Composer) -> String {
        composer.undecorate(&self.emit_to_string_internal(composer))
    }

    fn emit_to_string_internal(&self, composer: &Composer) -> String {
        let path = self.source.path(composer);
        let source = self.source.source(composer);
        let source_offset = self.source.offset();

        trace!("source:\n~{}~", source);
        trace!("source offset: {}", source_offset);

        let map_span = |rng: Range<usize>| -> Range<usize> {
            ((rng.start & ((1 << SPAN_SHIFT) - 1)).saturating_sub(source_offset))
                ..((rng.end & ((1 << SPAN_SHIFT) - 1)).saturating_sub(source_offset))
        };

        let files = SimpleFile::new(path, source);
        let config = term::Config::default();
        let mut writer = term::termcolor::Ansi::new(Vec::new());

        let (labels, notes) = match &self.inner {
            ComposerErrorInner::DecorationInSource(range) => {
                (vec![Label::primary((), range.clone())], vec![])
            }
            ComposerErrorInner::HeaderValidationError(v)
            | ComposerErrorInner::ShaderValidationError(v) => (
                v.spans()
                    .map(|(span, desc)| {
                        trace!(
                            "mapping span {:?} -> {:?}",
                            span.to_range().unwrap(),
                            map_span(span.to_range().unwrap_or(0..0))
                        );
                        Label::primary((), map_span(span.to_range().unwrap_or(0..0)))
                            .with_message(desc.to_owned())
                    })
                    .collect(),
                ErrorSources::of(&v)
                    .map(|source| source.to_string())
                    .collect(),
            ),
            ComposerErrorInner::ImportNotFound(msg, pos) => (
                vec![Label::primary((), *pos..*pos)],
                vec![format!("missing import '{msg}'")],
            ),
            ComposerErrorInner::WgslParseError(e) => (
                e.labels()
                    .map(|(range, msg)| {
                        Label::primary((), map_span(range.to_range().unwrap())).with_message(msg)
                    })
                    .collect(),
                vec![e.message().to_owned()],
            ),
            ComposerErrorInner::GlslParseError(e) => (
                e.iter()
                    .map(|naga::front::glsl::Error { kind, meta }| {
                        Label::primary((), map_span(meta.to_range().unwrap_or(0..0)))
                            .with_message(kind.to_string())
                    })
                    .collect(),
                vec![],
            ),
            ComposerErrorInner::NotEnoughEndIfs(pos)
            | ComposerErrorInner::TooManyEndIfs(pos)
            | ComposerErrorInner::UnknownShaderDef { pos, .. }
            | ComposerErrorInner::UnknownShaderDefOperator { pos, .. }
            | ComposerErrorInner::InvalidShaderDefComparisonValue { pos, .. }
            | ComposerErrorInner::OverrideNotVirtual { pos, .. }
            | ComposerErrorInner::GlslInvalidVersion(pos) => {
                (vec![Label::primary((), *pos..*pos)], vec![])
            }
            ComposerErrorInner::WgslBackError(e) => {
                return format!("{path}: wgsl back error: {e}");
            }
            ComposerErrorInner::GlslBackError(e) => {
                return format!("{path}: glsl back error: {e}");
            }
            ComposerErrorInner::InconsistentShaderDefValue { def } => {
                return format!(
                    "{path}: multiple inconsistent shader def values: '{def}'"
                );
            }
            ComposerErrorInner::RedirectError(..) => (
                vec![Label::primary((), 0..0)],
                vec![format!("override error")],
            ),
            ComposerErrorInner::NoModuleName => {
                return format!(
                    "{path}: no #define_import_path declaration found in composable module"
                );
            }
            ComposerErrorInner::InvalidIdentifier { at, .. } => (
                vec![Label::primary((), map_span(at.to_range().unwrap_or(0..0)))
                    .with_message(self.inner.to_string())],
                vec![],
            ),
        };

        let diagnostic = Diagnostic::error()
            .with_message(self.inner.to_string())
            .with_labels(labels)
            .with_notes(notes);

        term::emit(&mut writer, &config, &files, &diagnostic).expect("cannot write error");

        let msg = writer.into_inner();
        let msg = String::from_utf8_lossy(&msg);

        msg.to_string()
    }
}

/// module composer.
/// stores any modules that can be imported into a shader
/// and builds the final shader
#[derive(Debug)]
pub struct Composer {
    pub validate: bool,
    pub module_sets: HashMap<String, ComposableModuleDefinition>,
    pub module_index: HashMap<usize, String>,
    pub capabilities: naga::valid::Capabilities,
    check_decoration_regex: Regex,
    undecorate_regex: Regex,
    version_regex: Regex,
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    ifop_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
    def_regex: Regex,
    def_regex_delimited: Regex,
    import_custom_path_as_regex: Regex,
    import_custom_path_regex: Regex,
    import_items_regex: Regex,
    identifier_regex: Regex,
    annotated_identifier_regex: Regex,
    define_import_path_regex: Regex,
    virtual_fn_regex: Regex,
    override_fn_regex: Regex,
    undecorate_override_regex: Regex,
    auto_binding_regex: Regex,
    auto_binding_index: u32,
}

// shift for module index
// 21 gives
//   max size for shader of 2m characters
//   max 2048 modules
const SPAN_SHIFT: usize = 21;

impl Default for Composer {
    fn default() -> Self {
        Self {
            validate: true,
            capabilities: Default::default(),
            module_sets: Default::default(),
            module_index: Default::default(),
            check_decoration_regex: Regex::new(format!("({}|{})", regex_syntax::escape(DECORATION_PRE), regex_syntax::escape(DECORATION_OVERRIDE_PRE)).as_str()).unwrap(),
            undecorate_regex: Regex::new(
                format!(
                    "{}([A-Z0-9]*){}",
                    regex_syntax::escape(DECORATION_PRE),
                    regex_syntax::escape(DECORATION_POST)
                )
                .as_str(),
            )
            .unwrap(),
            version_regex: Regex::new(r"^\s*#version\s+([0-9]+)").unwrap(),
            ifdef_regex: Regex::new(r"^\s*#\s*ifdef\s+([\w|\d|_]+)").unwrap(),
            ifndef_regex: Regex::new(r"^\s*#\s*ifndef\s+([\w|\d|_]+)").unwrap(),
            ifop_regex: Regex::new(r"^\s*#\s*if\s+([\w|\d|_]+)\s*([^\s]*)\s*([\w|\d]+)").unwrap(),
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
            def_regex: Regex::new(r"#\s*([\w|\d|_]+)").unwrap(),
            def_regex_delimited: Regex::new(r"#\s*\{([\w|\d|_]+)\}").unwrap(),            
            import_custom_path_as_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)\s+as\s+([^\s]+)")
                .unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)").unwrap(),
            import_items_regex: Regex::new(r"^\s*#\s*from\s+([^\s]+)\s+import\s*((?:[\w|\d|_]+)(?:\s*,\s*[\w|\d|_]+)*)").unwrap(),
            identifier_regex: Regex::new(r"([\w|\d|_]+)").unwrap(),
            annotated_identifier_regex: Regex::new(r"([^\w|^]+)::([\w|\d|_]+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+([^\s]+)").unwrap(),
            virtual_fn_regex: Regex::new(r"(?P<lead>[\s]*virtual\s+fn\s+)(?P<function>[^\s]+)(?P<trail>\s*)\(").unwrap(),
            override_fn_regex: Regex::new(
                format!(
                    r"(?P<lead>[\s]*override\s+fn\s*){}(?P<module>[^\s]+){}(?P<function>[^\s]+)(?P<trail>\s*)\(", 
                    regex_syntax::escape(DECORATION_PRE),
                    regex_syntax::escape(DECORATION_POST)
                )
                .as_str()
            ).unwrap(),
            undecorate_override_regex: Regex::new(
                format!(
                    "{}([A-Z0-9]*){}",
                    regex_syntax::escape(DECORATION_OVERRIDE_PRE),
                    regex_syntax::escape(DECORATION_POST)
                )
                .as_str(),
            )
            .unwrap(),
            auto_binding_regex: Regex::new(r"@binding\(auto\)").unwrap(),
            auto_binding_index: 0,
        }
    }
}

const DECORATION_PRE: &str = "_naga_oil_mod_";
const DECORATION_POST: &str = "_member";

// must be same length as DECORATION_PRE for spans to work
const DECORATION_OVERRIDE_PRE: &str = "_naga_oil_vrt_";

struct IrBuildResult {
    module: naga::Module,
    start_offset: usize,
    override_functions: HashMap<String, Vec<String>>,
}

impl Composer {
    pub fn decorated_name(module_name: Option<&str>, item_name: &str) -> String {
        match module_name {
            Some(module_name) => format!("{}{}", Self::decorate(module_name), item_name),
            None => item_name.to_owned(),
        }
    }

    fn decorate(as_name: &str) -> String {
        let as_name = data_encoding::BASE32_NOPAD.encode(as_name.as_bytes());
        format!("{DECORATION_PRE}{as_name}{DECORATION_POST}")
    }

    fn decode(from: &str) -> String {
        String::from_utf8(data_encoding::BASE32_NOPAD.decode(from.as_bytes()).unwrap()).unwrap()
    }

    fn undecorate(&self, string: &str) -> String {
        let undecor = self
            .undecorate_regex
            .replace_all(string, |caps: &regex::Captures| {
                format!("{}::", Self::decode(caps.get(1).unwrap().as_str()))
            });

        let undecor =
            self.undecorate_override_regex
                .replace_all(&undecor, |caps: &regex::Captures| {
                    format!(
                        "override fn {}::",
                        Self::decode(caps.get(1).unwrap().as_str())
                    )
                });

        undecor.to_string()
    }

    fn sanitize_and_substitute_shader_string(
        &mut self,
        source: &str,
        imports: &[ImportDefWithOffset],
    ) -> Result<String, ComposerErrorInner> {
        let mut substituted_source = source.replace("\r\n", "\n").replace('\r', "\n");
        if !substituted_source.ends_with('\n') {
            substituted_source.push('\n');
        }

        // sort imports by decreasing length so we don't accidentally replace substrings of a longer import
        let mut imports = imports.to_vec();
        imports.sort_by_key(|import| usize::MAX - import.definition.as_name().len());

        let mut imported_items = HashMap::new();

        for import in imports {
            match import.definition.items {
                Some(items) => {
                    // gather individual imported items
                    for item in &items {
                        imported_items.insert(
                            item.clone(),
                            format!("{}{}", Self::decorate(&import.definition.import), item),
                        );
                    }
                }
                None => {
                    // replace the module name directly
                    substituted_source = substituted_source.replace(
                        format!("{}::", import.definition.as_name()).as_str(),
                        &Self::decorate(&import.definition.import),
                    );
                }
            }
        }

        // map individually imported items
        let mut error = None;
        struct MapReplacer<'a> {
            items: HashMap<String, String>,
            error: &'a mut Option<String>,
        }

        impl<'a> regex::Replacer for MapReplacer<'a> {
            fn replace_append(&mut self, cap: &regex::Captures<'_>, dst: &mut String) {
                let item = cap.get(2).unwrap().as_str();
                match self.items.get(item) {
                    Some(replacement) => {
                        dst.push_str(&format!("{}{}", cap.get(1).unwrap().as_str(), replacement));
                    }
                    None => *self.error = Some(item.to_string()),
                }
            }
        }

        let map_replacer = MapReplacer {
            items: imported_items,
            error: &mut error,
        };
        substituted_source = self
            .annotated_identifier_regex
            .replace_all(&substituted_source, map_replacer)
            .to_string();

        if let Some(error) = error {
            return Err(ComposerErrorInner::ImportNotFound(error, 0));
        }

        // replace @binding(auto) with an incrementing index
        struct AutoBindingReplacer<'a> {
            auto: &'a mut u32,
        }

        impl<'a> regex::Replacer for AutoBindingReplacer<'a> {
            fn replace_append(&mut self, _: &regex::Captures<'_>, dst: &mut String) {
                dst.push_str(&format!("@binding({})", self.auto));
                *self.auto += 1;
            }
        }

        let substituted_source = self.auto_binding_regex.replace_all(
            &substituted_source,
            AutoBindingReplacer {
                auto: &mut self.auto_binding_index,
            },
        );

        Ok(substituted_source.into_owned())
    }

    // build naga module for a given shader_def configuration. builds a minimal self-contained module built against headers for imports
    fn create_module_ir(
        &self,
        name: &str,
        source: String,
        language: ShaderLanguage,
        imports: &[ImportDefinition],
        shader_defs: &HashMap<String, ShaderDefValue>,
    ) -> Result<IrBuildResult, ComposerError> {
        debug!("creating IR for {} with defs: {:?}", name, shader_defs);

        let mut module_string = match language {
            ShaderLanguage::Wgsl => String::new(),
            ShaderLanguage::Glsl => String::from("#version 450\n"),
        };

        let mut override_functions: HashMap<String, Vec<String>> = HashMap::default();
        let mut added_imports: HashSet<String> = HashSet::new();

        for import in imports {
            if added_imports.contains(&import.import) {
                continue;
            }
            added_imports.insert(import.import.clone());
            // // we must have ensured these exist with Composer::ensure_imports()
            trace!("looking for {}", import.import);
            let import_module_set = self.module_sets.get(&import.import).unwrap();
            trace!("with defs {:?}", shader_defs);
            let module = import_module_set.get_module(shader_defs).unwrap();
            trace!("ok");

            // add header string
            module_string.push_str(module.headers.get(&language).unwrap().as_str());

            // gather overrides
            if !module.override_functions.is_empty() {
                for (original, replacements) in &module.override_functions {
                    match override_functions.entry(original.clone()) {
                        Entry::Occupied(o) => {
                            let existing = o.into_mut();
                            let new_replacements: Vec<_> = replacements
                                .iter()
                                .filter(|rep| !existing.contains(rep))
                                .cloned()
                                .collect();
                            existing.extend(new_replacements);
                        }
                        Entry::Vacant(v) => {
                            v.insert(replacements.clone());
                        }
                    }
                }
            }
        }

        let start_offset = module_string.len();

        module_string.push_str(&source);

        trace!(
            "parsing {}: {}, header len {}, total len {}",
            name,
            module_string,
            start_offset,
            module_string.len()
        );
        let module = match language {
            ShaderLanguage::Wgsl => naga::front::wgsl::parse_str(&module_string).map_err(|e| {
                debug!("full err'd source file: \n---\n{}\n---", module_string);
                ComposerError {
                    inner: ComposerErrorInner::WgslParseError(e),
                    source: ErrSource::Module(name.to_owned(), start_offset),
                }
            })?,
            ShaderLanguage::Glsl => naga::front::glsl::Parser::default()
                .parse(
                    &naga::front::glsl::Options {
                        stage: naga::ShaderStage::Vertex,
                        defines: Default::default(),
                    },
                    &module_string,
                )
                .map_err(|e| {
                    debug!("full err'd source file: \n---\n{}\n---", module_string);
                    ComposerError {
                        inner: ComposerErrorInner::GlslParseError(e),
                        source: ErrSource::Module(name.to_owned(), start_offset),
                    }
                })?,
        };

        Ok(IrBuildResult {
            module,
            start_offset,
            override_functions,
        })
    }

    // process #if[(n)?def]? / #else / #endif preprocessor directives,
    // strip module name and imports
    // also strip "#version xxx"
    fn preprocess_defs(
        &self,
        shader_str: &str,
        shader_defs: &HashMap<String, ShaderDefValue>,
        mut validate_len: bool,
    ) -> Result<(Option<String>, String, Vec<ImportDefWithOffset>), ComposerErrorInner> {
        let mut imports = Vec::new();
        let mut scopes = vec![true];
        let mut final_string = String::new();
        let mut name = None;
        let mut offset = 0;

        #[cfg(debug)]
        let len = shader_str.len();

        // this code broadly stolen from bevy_render::ShaderProcessor
        for line in shader_str.lines() {
            let mut output = false;
            if let Some(cap) = self.version_regex.captures(line) {
                let v = cap.get(1).unwrap().as_str();
                if v != "440" && v != "450" {
                    return Err(ComposerErrorInner::GlslInvalidVersion(offset));
                }
            } else if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && shader_defs.contains_key(def.as_str()));
            } else if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                scopes.push(*scopes.last().unwrap() && !shader_defs.contains_key(def.as_str()));
            } else if let Some(cap) = self.ifop_regex.captures(line) {
                let def = cap.get(1).unwrap();
                let op = cap.get(2).unwrap();
                let val = cap.get(3).unwrap();

                fn act_on<T: Eq + Ord>(
                    a: T,
                    b: T,
                    op: &str,
                    pos: usize,
                ) -> Result<bool, ComposerErrorInner> {
                    match op {
                        "==" => Ok(a == b),
                        "!=" => Ok(a != b),
                        ">" => Ok(a > b),
                        ">=" => Ok(a >= b),
                        "<" => Ok(a < b),
                        "<=" => Ok(a <= b),
                        _ => Err(ComposerErrorInner::UnknownShaderDefOperator {
                            pos,
                            operator: op.to_string(),
                        }),
                    }
                }

                let def_value =
                    shader_defs
                        .get(def.as_str())
                        .ok_or(ComposerErrorInner::UnknownShaderDef {
                            pos: offset,
                            shader_def_name: def.as_str().to_string(),
                        })?;
                let new_scope = match def_value {
                    ShaderDefValue::Bool(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "bool".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                    ShaderDefValue::Int(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "int".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                    ShaderDefValue::UInt(def_value) => {
                        let val = val.as_str().parse().map_err(|_| {
                            ComposerErrorInner::InvalidShaderDefComparisonValue {
                                pos: offset,
                                shader_def_name: def.as_str().to_string(),
                                value: val.as_str().to_string(),
                                expected: "int".to_string(),
                            }
                        })?;
                        act_on(*def_value, val, op.as_str(), offset)?
                    }
                };
                scopes.push(*scopes.last().unwrap() && new_scope);
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
                    return Err(ComposerErrorInner::TooManyEndIfs(offset));
                }
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            } else if *scopes.last().unwrap() {
                if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: Some(cap.get(2).unwrap().as_str().to_string()),
                            items: Default::default(),
                        },
                        offset,
                    });
                } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: None,
                            items: Default::default(),
                        },
                        offset,
                    });
                } else if let Some(cap) = self.import_items_regex.captures(line) {
                    imports.push(ImportDefWithOffset {
                        definition: ImportDefinition {
                            import: cap.get(1).unwrap().as_str().to_string(),
                            as_name: None,
                            items: Some(
                                self.identifier_regex
                                    .captures_iter(cap.get(2).unwrap().as_str())
                                    .map(|ident_cap| ident_cap.get(1).unwrap().as_str().to_owned())
                                    .collect(),
                            ),
                        },
                        offset,
                    });
                } else {
                    let mut line_with_defs = line.to_string();
                    for capture in self.def_regex.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    for capture in self.def_regex_delimited.captures_iter(line) {
                        let def = capture.get(1).unwrap();
                        if let Some(def) = shader_defs.get(def.as_str()) {
                            line_with_defs = self
                                .def_regex_delimited
                                .replace(&line_with_defs, def.value_as_string())
                                .to_string();
                        }
                    }
                    final_string.push_str(&line_with_defs);
                    let diff = line.len() as i32 - line_with_defs.len() as i32;
                    if diff > 0 {
                        final_string.extend(std::iter::repeat(" ").take(diff as usize));
                    } else if diff < 0 && validate_len {
                        // this sucks
                        warn!("source code map requires shader_def values to be no longer than the corresponding shader_def name, error reporting may not be correct:\noriginal: {}\nreplaced: {}", line, line_with_defs);
                        validate_len = false;
                    }
                    output = true;
                }
            }

            if !output {
                // output spaces for removed lines to keep spans consistent (errors report against substituted_source, which is not preprocessed)
                final_string.extend(std::iter::repeat(" ").take(line.len()));
            }
            final_string.push('\n');
            offset += line.len() + 1;
        }

        if scopes.len() != 1 {
            return Err(ComposerErrorInner::NotEnoughEndIfs(offset));
        }

        #[cfg(debug)]
        if validate_len {
            let revised_len = final_string.len();
            assert_eq!(len, revised_len);
        }

        Ok((name, final_string, imports))
    }

    // extract module name and imports
    fn get_preprocessor_data(
        &self,
        shader_str: &str,
    ) -> (Option<String>, Vec<ImportDefWithOffset>) {
        let mut imports = Vec::new();
        let mut name = None;
        let mut offset = 0;
        for line in shader_str.lines() {
            if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: Some(cap.get(2).unwrap().as_str().to_string()),
                        items: Default::default(),
                    },
                    offset,
                });
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: None,
                        items: Default::default(),
                    },
                    offset,
                });
            } else if let Some(cap) = self.import_items_regex.captures(line) {
                imports.push(ImportDefWithOffset {
                    definition: ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: None,
                        items: Some(
                            self.identifier_regex
                                .captures_iter(cap.get(2).unwrap().as_str())
                                .map(|ident_cap| ident_cap.get(1).unwrap().as_str().to_owned())
                                .collect(),
                        ),
                    },
                    offset,
                });
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            }

            offset += line.len() + 1;
        }

        (name, imports)
    }

    // check that identifiers exported by a module do not get modified in string export
    fn validate_identifiers(
        source_ir: &naga::Module,
        lang: ShaderLanguage,
        header: &str,
        module_decoration: &str,
        owned_types: &HashSet<String>,
    ) -> Result<(), ComposerErrorInner> {
        // TODO: remove this once glsl has INCLUDE_UNUSED_ITEMS
        if lang == ShaderLanguage::Glsl {
            return Ok(());
        }

        let recompiled = match lang {
            ShaderLanguage::Wgsl => naga::front::wgsl::parse_str(header).unwrap(),
            ShaderLanguage::Glsl => naga::front::glsl::Parser::default()
                .parse(
                    &naga::front::glsl::Options {
                        stage: naga::ShaderStage::Vertex,
                        defines: Default::default(),
                    },
                    &format!("{}\n{}", header, "void main() {}"),
                )
                .unwrap(),
        };

        let recompiled_types: HashMap<_, _> = recompiled
            .types
            .iter()
            .flat_map(|(h, ty)| ty.name.as_deref().map(|name| (name, h)))
            .collect();
        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                let decorated_type_name = format!("{module_decoration}{name}");
                if !owned_types.contains(&decorated_type_name) {
                    continue;
                }
                match recompiled_types.get(decorated_type_name.as_str()) {
                    Some(recompiled_h) => {
                        if let naga::TypeInner::Struct { members, .. } = &ty.inner {
                            let recompiled_ty = recompiled.types.get_handle(*recompiled_h).unwrap();
                            let naga::TypeInner::Struct { members: recompiled_members, .. } = &recompiled_ty.inner else {
                                panic!();
                            };
                            for (member, recompiled_member) in
                                members.iter().zip(recompiled_members)
                            {
                                if member.name != recompiled_member.name {
                                    return Err(ComposerErrorInner::InvalidIdentifier {
                                        original: member.name.clone().unwrap_or_default(),
                                        at: source_ir.types.get_span(h),
                                    });
                                }
                            }
                        }
                    }
                    None => {
                        return Err(ComposerErrorInner::InvalidIdentifier {
                            original: name.clone(),
                            at: source_ir.types.get_span(h),
                        })
                    }
                }
            }
        }

        let recompiled_consts: HashSet<_> = recompiled
            .constants
            .iter()
            .flat_map(|(_, c)| c.name.as_deref())
            .filter(|name| name.starts_with(module_decoration))
            .collect();
        for (h, c) in source_ir.constants.iter() {
            if let Some(name) = &c.name {
                if name.starts_with(module_decoration) && !recompiled_consts.contains(name.as_str())
                {
                    return Err(ComposerErrorInner::InvalidIdentifier {
                        original: name.clone(),
                        at: source_ir.constants.get_span(h),
                    });
                }
            }
        }

        let recompiled_globals: HashSet<_> = recompiled
            .global_variables
            .iter()
            .flat_map(|(_, c)| c.name.as_deref())
            .filter(|name| name.starts_with(module_decoration))
            .collect();
        for (h, gv) in source_ir.global_variables.iter() {
            if let Some(name) = &gv.name {
                if name.starts_with(module_decoration)
                    && !recompiled_globals.contains(name.as_str())
                {
                    return Err(ComposerErrorInner::InvalidIdentifier {
                        original: name.clone(),
                        at: source_ir.global_variables.get_span(h),
                    });
                }
            }
        }

        let recompiled_fns: HashSet<_> = recompiled
            .functions
            .iter()
            .flat_map(|(_, c)| c.name.as_deref())
            .filter(|name| name.starts_with(module_decoration))
            .collect();
        for (h, f) in source_ir.functions.iter() {
            if let Some(name) = &f.name {
                if name.starts_with(module_decoration) && !recompiled_fns.contains(name.as_str()) {
                    return Err(ComposerErrorInner::InvalidIdentifier {
                        original: name.clone(),
                        at: source_ir.functions.get_span(h),
                    });
                }
            }
        }

        Ok(())
    }

    // build a ComposableModule from a ComposableModuleDefinition, for a given set of shader defs
    // - build the naga IR (against headers)
    // - record any types/vars/constants/functions that are defined within this module
    // - build headers for each supported language
    fn create_composable_module(
        &self,
        module_definition: &ComposableModuleDefinition,
        module_decoration: String,
        shader_defs: &HashMap<String, ShaderDefValue>,
        create_headers: bool,
        demote_entrypoints: bool,
    ) -> Result<ComposableModule, ComposerError> {
        let wrap_err = |inner: ComposerErrorInner| -> ComposerError {
            ComposerError {
                inner,
                source: ErrSource::Module(module_definition.name.to_owned(), 0),
            }
        };

        let (_, source, imports) = self
            .preprocess_defs(
                &module_definition.substituted_source,
                shader_defs,
                self.validate,
            )
            .map_err(wrap_err)?;

        let mut imports: Vec<_> = imports
            .into_iter()
            .map(|import_with_offset| import_with_offset.definition)
            .collect();
        imports.extend(module_definition.additional_imports.to_vec());

        trace!(
            "create composable module {}: source len {}",
            module_definition.name,
            source.len()
        );

        // record virtual/overridable functions
        let mut virtual_functions: HashSet<String> = Default::default();
        let source = self
            .virtual_fn_regex
            .replace_all(&source, |cap: &regex::Captures| {
                let target_function = cap.get(2).unwrap().as_str().to_owned();

                let replacement_str = format!(
                    "{}fn {}{}(",
                    " ".repeat(cap.get(1).unwrap().range().len() - 3),
                    target_function,
                    " ".repeat(cap.get(3).unwrap().range().len()),
                );

                virtual_functions.insert(target_function);

                replacement_str
            });

        // record and rename override functions
        let mut local_override_functions: HashMap<String, String> = Default::default();

        #[cfg(not(feature = "override_any"))]
        let mut override_error = None;

        let source =
            self.override_fn_regex
                .replace_all(&source, |cap: &regex::Captures| {
                    let target_module = cap.get(2).unwrap().as_str().to_owned();
                    let target_function = cap.get(3).unwrap().as_str().to_owned();

                    #[cfg(not(feature = "override_any"))]
                    {
                        // ensure overrides are applied to virtual functions
                        let raw_module_name = Self::decode(&target_module);
                        let module_set = self.module_sets.get(&raw_module_name);

                        match module_set {
                            None => {
                                let pos = cap.get(2).unwrap().start();
                                override_error = Some(wrap_err(
                                    ComposerErrorInner::ImportNotFound(raw_module_name, pos),
                                ));
                            }
                            Some(module_set) => {
                                let module = module_set.get_module(shader_defs).unwrap();
                                if !module.virtual_functions.contains(&target_function) {
                                    let pos = cap.get(3).unwrap().start();
                                    override_error =
                                        Some(wrap_err(ComposerErrorInner::OverrideNotVirtual {
                                            name: target_function.clone(),
                                            pos,
                                        }));
                                }
                            }
                        }
                    }

                    let base_name = format!(
                        "{}{}{}{}",
                        DECORATION_PRE,
                        target_module.as_str(),
                        DECORATION_POST,
                        target_function.as_str()
                    );
                    let rename = format!(
                        "{}{}{}{}",
                        DECORATION_OVERRIDE_PRE,
                        target_module.as_str(),
                        DECORATION_POST,
                        target_function.as_str()
                    );

                    let replacement_str = format!(
                        "{}fn {}{}(",
                        " ".repeat(cap.get(1).unwrap().range().len() - 3),
                        rename,
                        " ".repeat(cap.get(4).unwrap().range().len()),
                    );

                    local_override_functions.insert(rename, base_name);

                    replacement_str
                })
                .to_string();

        #[cfg(not(feature = "override_any"))]
        if let Some(err) = override_error {
            return Err(err);
        }

        trace!("local overrides: {:?}", local_override_functions);
        trace!(
            "create composable module {}: source len {}",
            module_definition.name,
            source.len()
        );

        let IrBuildResult {
            module: mut source_ir,
            start_offset,
            mut override_functions,
        } = self.create_module_ir(
            &module_definition.name,
            source,
            module_definition.language,
            &imports,
            shader_defs,
        )?;

        // from here on errors need to be reported using the modified source with start_offset
        let wrap_err = |inner: ComposerErrorInner| -> ComposerError {
            ComposerError {
                inner,
                source: ErrSource::Module(module_definition.name.to_owned(), start_offset),
            }
        };

        // add our local override to the total set of overrides for the given function
        for (rename, base_name) in &local_override_functions {
            override_functions
                .entry(base_name.clone())
                .or_default()
                .push(format!("{module_decoration}{rename}"));
        }

        // rename and record owned items (except types which can't be mutably accessed)
        let mut owned_constants = HashMap::new();
        for (h, c) in source_ir.constants.iter_mut() {
            if let Some(name) = c.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{module_decoration}{name}");
                    owned_constants.insert(name.clone(), h);
                }
            }
        }

        let mut owned_vars = HashMap::new();
        for (h, gv) in source_ir.global_variables.iter_mut() {
            if let Some(name) = gv.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{module_decoration}{name}");

                    owned_vars.insert(name.clone(), h);
                }
            }
        }

        let mut owned_functions = HashMap::new();
        for (h_f, f) in source_ir.functions.iter_mut() {
            if let Some(name) = f.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{module_decoration}{name}");

                    // create dummy header function
                    let header_function = naga::Function {
                        name: Some(name.clone()),
                        arguments: f.arguments.to_vec(),
                        result: f.result.clone(),
                        local_variables: Default::default(),
                        expressions: Default::default(),
                        named_expressions: Default::default(),
                        body: Default::default(),
                    };

                    // record owned function
                    owned_functions.insert(name.clone(), (Some(h_f), header_function));
                }
            }
        }

        if demote_entrypoints {
            // make normal functions out of the source entry points
            for ep in &mut source_ir.entry_points {
                ep.function.name = Some(format!(
                    "{}{}",
                    module_decoration,
                    ep.function.name.as_deref().unwrap_or("main")
                ));
                let header_function = naga::Function {
                    name: ep.function.name.clone(),
                    arguments: ep
                        .function
                        .arguments
                        .iter()
                        .cloned()
                        .map(|arg| naga::FunctionArgument {
                            name: arg.name,
                            ty: arg.ty,
                            binding: None,
                        })
                        .collect(),
                    result: ep.function.result.clone().map(|res| naga::FunctionResult {
                        ty: res.ty,
                        binding: None,
                    }),
                    local_variables: Default::default(),
                    expressions: Default::default(),
                    named_expressions: Default::default(),
                    body: Default::default(),
                };

                owned_functions.insert(ep.function.name.clone().unwrap(), (None, header_function));
            }
        };

        let mut module_builder = DerivedModule::default();
        let mut header_builder = DerivedModule::default();
        module_builder.set_shader_source(&source_ir, 0);
        header_builder.set_shader_source(&source_ir, 0);

        let mut owned_types = HashSet::new();
        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                if !name.contains(DECORATION_PRE) {
                    let name = format!("{module_decoration}{name}");
                    owned_types.insert(name.clone());
                    // copy and rename types
                    module_builder.rename_type(&h, Some(name.clone()));
                    header_builder.rename_type(&h, Some(name));
                    continue;
                }
            }

            // copy all required types
            module_builder.import_type(&h);
        }

        // copy owned types into header and module
        for h in owned_constants.values() {
            header_builder.import_const(h);
            module_builder.import_const(h);
        }

        for h in owned_vars.values() {
            header_builder.import_global(h);
            module_builder.import_global(h);
        }

        // only stubs of owned functions into the header
        for (h_f, f) in owned_functions.values() {
            let span = h_f
                .map(|h_f| source_ir.functions.get_span(h_f))
                .unwrap_or(naga::Span::UNDEFINED);
            header_builder.import_function(f, span); // header stub function
        }
        // all functions into the module (note source_ir only contains stubs for imported functions)
        for (h_f, f) in source_ir.functions.iter() {
            let span = source_ir.functions.get_span(h_f);
            module_builder.import_function(f, span);
        }
        // // including entry points as vanilla functions if required
        if demote_entrypoints {
            for ep in &source_ir.entry_points {
                let mut f = ep.function.clone();
                f.arguments = f
                    .arguments
                    .into_iter()
                    .map(|arg| naga::FunctionArgument {
                        name: arg.name,
                        ty: arg.ty,
                        binding: None,
                    })
                    .collect();
                f.result = f.result.map(|res| naga::FunctionResult {
                    ty: res.ty,
                    binding: None,
                });

                module_builder.import_function(&f, naga::Span::UNDEFINED);
                // todo figure out how to get span info for entrypoints
            }
        }

        let mut module_ir = module_builder.into_module_with_entrypoints();
        let mut header_ir: naga::Module = header_builder.into();

        // rescope any imported constants
        let mut renamed_consts = HashMap::new();
        for (_, constant) in header_ir.constants.iter_mut() {
            if let Some(name) = constant.name.as_mut() {
                if name.contains(DECORATION_PRE) && !name.contains(&module_decoration) {
                    let rename = format!("{module_decoration}{name}");
                    trace!(
                        "{}: header rename {} -> {}",
                        module_definition.name,
                        name,
                        rename
                    );
                    renamed_consts.insert(name.clone(), rename.clone());
                    *name = rename;
                }
            }
        }
        // we do this in the module_ir, and source_ir as well so that identifier validation works
        for ir in [&mut module_ir, &mut source_ir] {
            for (_, constant) in ir.constants.iter_mut() {
                if let Some(name) = constant.name.as_mut() {
                    if let Some(rename) = renamed_consts.get(name) {
                        trace!(
                            "{}: module rename {} -> {}",
                            module_definition.name,
                            name,
                            rename
                        );
                        *name = rename.clone();
                    }
                }
            }
        }

        let info =
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), self.capabilities)
                .validate(&header_ir)
                .map_err(|e| wrap_err(ComposerErrorInner::HeaderValidationError(e)))?;

        let headers: HashMap<_, _> = if create_headers {
            [
                (
                    ShaderLanguage::Wgsl,
                    naga::back::wgsl::write_string(
                        &header_ir,
                        &info,
                        naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
                    )
                    .map_err(|e| wrap_err(ComposerErrorInner::WgslBackError(e)))?,
                ),
                (
                    // note this must come last as we add a dummy entry point
                    ShaderLanguage::Glsl,
                    {
                        // add a dummy entry point for glsl headers
                        let dummy_entry_point =
                            format!("{module_decoration}dummy_module_entry_point");
                        let func = naga::Function {
                            name: Some(dummy_entry_point.clone()),
                            arguments: Default::default(),
                            result: None,
                            local_variables: Default::default(),
                            expressions: Default::default(),
                            named_expressions: Default::default(),
                            body: Default::default(),
                        };
                        let ep = EntryPoint {
                            name: dummy_entry_point.clone(),
                            stage: naga::ShaderStage::Vertex,
                            function: func,
                            early_depth_test: None,
                            workgroup_size: [0, 0, 0],
                        };

                        header_ir.entry_points.push(ep);

                        let info = naga::valid::Validator::new(
                            naga::valid::ValidationFlags::all(),
                            self.capabilities,
                        )
                        .validate(&header_ir)
                        .map_err(|e| wrap_err(ComposerErrorInner::HeaderValidationError(e)))?;

                        let mut string = String::new();
                        let options = naga::back::glsl::Options {
                            version: naga::back::glsl::Version::Desktop(450),
                            writer_flags: naga::back::glsl::WriterFlags::empty(),
                            ..Default::default()
                        };
                        let pipeline_options = naga::back::glsl::PipelineOptions {
                            shader_stage: naga::ShaderStage::Vertex,
                            entry_point: dummy_entry_point,
                            multiview: None,
                        };
                        let mut writer = naga::back::glsl::Writer::new(
                            &mut string,
                            &header_ir,
                            &info,
                            &options,
                            &pipeline_options,
                            naga::proc::BoundsCheckPolicies::default(),
                        )
                        .map_err(|e| ComposerError {
                            inner: ComposerErrorInner::GlslBackError(e),
                            source: ErrSource::Module(
                                module_definition.name.to_owned(),
                                start_offset,
                            ),
                        })?;
                        writer.write().map_err(|e| ComposerError {
                            inner: ComposerErrorInner::GlslBackError(e),
                            source: ErrSource::Module(
                                module_definition.name.to_owned(),
                                start_offset,
                            ),
                        })?;
                        // strip version decl and main() impl
                        let lines: Vec<_> = string.lines().collect();
                        let string = lines[1..lines.len() - 3].join("\n");
                        trace!(
                            "glsl header for {}:\n\"\n{:?}\n\"",
                            module_definition.name,
                            string
                        );
                        string
                    },
                ),
            ]
            .into()
        } else {
            Default::default()
        };

        if self.validate {
            // check that identifiers haven't been renamed
            for (lang, header) in headers.iter() {
                Self::validate_identifiers(
                    &source_ir,
                    *lang,
                    header,
                    &module_decoration,
                    &owned_types,
                )
                .map_err(wrap_err)?;
            }
        }

        let composable_module = ComposableModule {
            decorated_name: module_decoration,
            imports,
            owned_types,
            owned_constants: owned_constants.into_keys().collect(),
            owned_vars: owned_vars.into_keys().collect(),
            owned_functions: owned_functions.into_keys().collect(),
            virtual_functions,
            override_functions,
            module_ir,
            headers,
            start_offset,
        };

        Ok(composable_module)
    }

    // shunt all data owned by a composable into a derived module
    fn add_composable_data<'a>(
        derived: &mut DerivedModule<'a>,
        composable: &'a ComposableModule,
        items: Option<&Vec<String>>,
        span_offset: usize,
    ) {
        let items: Option<HashSet<String>> = items.map(|items| {
            items
                .iter()
                .map(|item| format!("{}{}", composable.decorated_name, item))
                .collect()
        });
        let items = items.as_ref();

        derived.set_shader_source(&composable.module_ir, span_offset);

        for (h, ty) in composable.module_ir.types.iter() {
            if let Some(name) = &ty.name {
                if composable.owned_types.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_type(&h);
                }
            }
        }

        for (h, c) in composable.module_ir.constants.iter() {
            if let Some(name) = &c.name {
                if composable.owned_constants.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_const(&h);
                }
            }
        }

        for (h, v) in composable.module_ir.global_variables.iter() {
            if let Some(name) = &v.name {
                if composable.owned_vars.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_global(&h);
                }
            }
        }

        for (h_f, f) in composable.module_ir.functions.iter() {
            if let Some(name) = &f.name {
                if composable.owned_functions.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    let span = composable.module_ir.functions.get_span(h_f);
                    derived.import_function_if_new(f, span);
                }
            }
        }

        derived.clear_shader_source();
    }

    // add an import (and recursive imports) into a derived module
    fn add_import<'a>(
        &'a self,
        derived: &mut DerivedModule<'a>,
        import: &ImportDefinition,
        shader_defs: &HashMap<String, ShaderDefValue>,
        already_added: &mut HashSet<String>,
    ) {
        if already_added.contains(&import.import) {
            trace!("skipping {}, already added", import.import);
            return;
        }

        if import.items.is_none() {
            already_added.insert(import.import.clone());
        }

        let import_module_set = self.module_sets.get(&import.import).unwrap();
        let module = import_module_set.get_module(shader_defs).unwrap();

        for import in &module.imports {
            self.add_import(derived, import, shader_defs, already_added);
        }

        Self::add_composable_data(
            derived,
            module,
            import.items.as_ref(),
            import_module_set.module_index << SPAN_SHIFT,
        );
    }

    fn ensure_import(
        &mut self,
        module_set: &ComposableModuleDefinition,
        shader_defs: &HashMap<String, ShaderDefValue>,
    ) -> Result<ComposableModule, ComposerError> {
        let (_, _, imports) = self
            .preprocess_defs(&module_set.substituted_source, shader_defs, self.validate)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Module(module_set.name.to_owned(), 0),
            })?;

        self.ensure_imports(imports.iter().map(|import| &import.definition), shader_defs)?;
        self.ensure_imports(&module_set.additional_imports, shader_defs)?;

        self.create_composable_module(
            module_set,
            Self::decorate(&module_set.name),
            shader_defs,
            true,
            true,
        )
    }

    // build required ComposableModules for a given set of shader_defs
    fn ensure_imports<'a>(
        &mut self,
        imports: impl IntoIterator<Item = &'a ImportDefinition>,
        shader_defs: &HashMap<String, ShaderDefValue>,
    ) -> Result<(), ComposerError> {
        for ImportDefinition { import, .. } in imports.into_iter() {
            // we've already ensured imports exist when they were added
            let module_set = self.module_sets.get(import).unwrap();
            if module_set.get_module(shader_defs).is_some() {
                continue;
            }

            // we need to build the module
            // take the set so we can recurse without borrowing
            let (set_key, mut module_set) = self.module_sets.remove_entry(import).unwrap();

            match self.ensure_import(&module_set, shader_defs) {
                Ok(module) => {
                    module_set.insert_module(shader_defs, module);
                    self.module_sets.insert(set_key, module_set);
                }
                Err(e) => {
                    self.module_sets.insert(set_key, module_set);
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct ComposableModuleDescriptor<'a> {
    pub source: &'a str,
    pub file_path: &'a str,
    pub language: ShaderLanguage,
    pub as_name: Option<String>,
    pub additional_imports: &'a [ImportDefinition],
    pub shader_defs: HashMap<String, ShaderDefValue>,
}

#[derive(Default)]
pub struct NagaModuleDescriptor<'a> {
    pub source: &'a str,
    pub file_path: &'a str,
    pub shader_type: ShaderType,
    pub shader_defs: HashMap<String, ShaderDefValue>,
    pub additional_imports: &'a [ImportDefinition],
}

// public api
impl Composer {
    /// create a non-validating composer.
    /// validation errors in the final shader will not be caught, and errors resulting from their
    /// use will have bad span data, so codespan reporting will fail.
    /// use default() to create a validating composer.
    pub fn non_validating() -> Self {
        Self {
            validate: false,
            ..Default::default()
        }
    }

    /// specify capabilities to be used for naga module generation.
    /// purges any existing modules
    pub fn with_capabilities(self, capabilities: naga::valid::Capabilities) -> Self {
        Self {
            capabilities,
            validate: self.validate,
            ..Default::default()
        }
    }

    /// check if a module with the given name has been added
    pub fn contains_module(&self, module_name: &str) -> bool {
        self.module_sets.contains_key(module_name)
    }

    /// add a composable module to the composer.
    /// all modules imported by this module must already have been added
    pub fn add_composable_module(
        &mut self,
        desc: ComposableModuleDescriptor,
    ) -> Result<&ComposableModuleDefinition, ComposerError> {
        let ComposableModuleDescriptor {
            source,
            file_path,
            language,
            as_name,
            additional_imports,
            mut shader_defs,
        } = desc;

        // reject a module containing the DECORATION strings
        if let Some(decor) = self.check_decoration_regex.find(source) {
            return Err(ComposerError {
                inner: ComposerErrorInner::DecorationInSource(decor.range()),
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            });
        }

        let (module_name, mut imports) = self.get_preprocessor_data(source);
        let module_name = as_name.or(module_name);
        if module_name.is_none() {
            return Err(ComposerError {
                inner: ComposerErrorInner::NoModuleName,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            });
        }
        let module_name = module_name.unwrap();

        debug!(
            "adding module definition for {} with defs: {:?}",
            module_name, shader_defs
        );

        // add custom imports
        let additional_imports = additional_imports.to_vec();
        imports.extend(
            additional_imports
                .iter()
                .cloned()
                .map(|def| ImportDefWithOffset {
                    definition: def,
                    offset: 0,
                }),
        );

        let substituted_source = self
            .sanitize_and_substitute_shader_string(source, &imports)
            .map_err(|e| ComposerError {
                inner: e,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            })?;

        let mut effective_defs = HashSet::new();
        for import in &imports {
            // we require modules already added so that we can capture the shader_defs that may impact us by impacting our dependencies
            let module_set = self
                .module_sets
                .get(&import.definition.import)
                .ok_or_else(|| ComposerError {
                    inner: ComposerErrorInner::ImportNotFound(
                        import.definition.import.clone(),
                        import.offset,
                    ),
                    source: ErrSource::Constructing {
                        path: file_path.to_owned(),
                        source: substituted_source.clone(),
                        offset: 0,
                    },
                })?;
            effective_defs.extend(module_set.effective_defs.iter().cloned());
            shader_defs.extend(
                module_set
                    .shader_defs
                    .iter()
                    .map(|def| (def.0.clone(), *def.1)),
            );
        }

        // record our explicit effective shader_defs
        for line in source.lines() {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str().to_owned());
            }
            if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str().to_owned());
            }
        }

        // remove defs that are already specified through our imports
        effective_defs.retain(|name| !shader_defs.contains_key(name));

        // can't gracefully report errors for more modules. perhaps this should be a warning
        assert!((self.module_sets.len() as u32) < u32::MAX >> SPAN_SHIFT);
        let module_index = self.module_sets.len() + 1;

        let module_set = ComposableModuleDefinition {
            name: module_name.clone(),
            substituted_source,
            file_path: file_path.to_owned(),
            language,
            effective_defs: effective_defs.into_iter().collect(),
            all_imports: imports.into_iter().map(|id| id.definition.import).collect(),
            additional_imports,
            shader_defs,
            module_index,
            modules: Default::default(),
        };

        // invalidate dependent modules if this module already exists
        self.remove_composable_module(&module_name);

        self.module_sets.insert(module_name.clone(), module_set);
        self.module_index.insert(module_index, module_name.clone());
        Ok(self.module_sets.get(&module_name).unwrap())
    }

    /// remove a composable module. also removes modules that depend on this module, as we cannot be sure about
    /// the completeness of their effective shader defs any more...
    pub fn remove_composable_module(&mut self, module_name: &str) {
        // todo this could be improved by making effective defs an Option<HashSet> and populating on demand?
        let mut dependent_sets = Vec::new();

        if self.module_sets.remove(module_name).is_some() {
            dependent_sets.extend(self.module_sets.iter().filter_map(|(dependent_name, set)| {
                if set.all_imports.contains(module_name) {
                    Some(dependent_name.clone())
                } else {
                    None
                }
            }));
        }

        for dependent_set in dependent_sets {
            self.remove_composable_module(&dependent_set);
        }
    }

    /// build a naga shader module
    pub fn make_naga_module(
        &mut self,
        desc: NagaModuleDescriptor,
    ) -> Result<naga::Module, ComposerError> {
        let NagaModuleDescriptor {
            source,
            file_path,
            shader_type,
            mut shader_defs,
            additional_imports,
        } = desc;
        let (name, modified_source, imports) = self
            .preprocess_defs(source, &shader_defs, false)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            })?;

        let name = name.unwrap_or_default();
        let substituted_source = self
            .sanitize_and_substitute_shader_string(source, &imports)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            })?;

        // make sure imports have been added
        // and gather additional defs specified at module level
        for (import_name, offset) in imports
            .iter()
            .map(|id| (&id.definition.import, id.offset))
            .chain(additional_imports.iter().map(|ai| (&ai.import, 0)))
        {
            if let Some(module_set) = self.module_sets.get(import_name) {
                for (def, value) in &module_set.shader_defs {
                    if let Some(prior_value) = shader_defs.insert(def.clone(), *value) {
                        if prior_value != *value {
                            return Err(ComposerError {
                                inner: ComposerErrorInner::InconsistentShaderDefValue {
                                    def: def.clone(),
                                },
                                source: ErrSource::Constructing {
                                    path: file_path.to_owned(),
                                    source: substituted_source,
                                    offset: 0,
                                },
                            });
                        }
                    }
                }
            } else {
                return Err(ComposerError {
                    inner: ComposerErrorInner::ImportNotFound(import_name.clone(), offset),
                    source: ErrSource::Constructing {
                        path: file_path.to_owned(),
                        source: substituted_source,
                        offset: 0,
                    },
                });
            }
        }
        self.ensure_imports(
            imports.iter().map(|import| &import.definition),
            &shader_defs,
        )?;
        self.ensure_imports(additional_imports, &shader_defs)?;

        let definition = ComposableModuleDefinition {
            name,
            substituted_source,
            language: shader_type.into(),
            file_path: file_path.to_owned(),
            module_index: 0,
            additional_imports: additional_imports.to_vec(),
            // we don't care about these for creating a top-level module
            effective_defs: Default::default(),
            all_imports: Default::default(),
            shader_defs: Default::default(),
            modules: Default::default(),
        };

        let composable = self
            .create_composable_module(&definition, String::from(""), &shader_defs, false, false)
            .map_err(|e| ComposerError {
                inner: e.inner,
                source: ErrSource::Constructing {
                    path: definition.file_path.to_owned(),
                    source: definition.substituted_source.to_owned(),
                    offset: e.source.offset(),
                },
            })?;

        let mut derived = DerivedModule::default();

        let mut already_added = Default::default();
        for import in &composable.imports {
            self.add_import(&mut derived, import, &shader_defs, &mut already_added);
        }

        Self::add_composable_data(&mut derived, &composable, None, 0);

        let stage = match shader_type {
            ShaderType::GlslVertex => Some(naga::ShaderStage::Vertex),
            ShaderType::GlslFragment => Some(naga::ShaderStage::Fragment),
            _ => None,
        };

        let mut entry_points = Vec::default();
        derived.set_shader_source(&composable.module_ir, 0);
        for ep in &composable.module_ir.entry_points {
            let mapped_func = derived.localize_function(&ep.function);
            entry_points.push(EntryPoint {
                name: ep.name.clone(),
                function: mapped_func,
                stage: stage.unwrap_or(ep.stage),
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
            });
        }

        let mut naga_module = naga::Module {
            entry_points,
            ..derived.into()
        };

        // apply overrides
        if !composable.override_functions.is_empty() {
            let mut redirect = Redirector::new(naga_module);

            for (base_function, overrides) in composable.override_functions {
                let mut omit = HashSet::default();

                let mut original = base_function;
                for replacement in overrides {
                    let (_h_orig, _h_replace) = redirect
                        .redirect_function(&original, &replacement, &omit)
                        .map_err(|e| ComposerError {
                            inner: e.into(),
                            source: ErrSource::Constructing {
                                path: file_path.to_owned(),
                                source: modified_source.clone(),
                                offset: composable.start_offset,
                            },
                        })?;
                    omit.insert(replacement.clone());
                    original = replacement;
                }
            }

            naga_module = redirect.into_module().map_err(|e| ComposerError {
                inner: e.into(),
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: modified_source.clone(),
                    offset: composable.start_offset,
                },
            })?;
        }

        // validation
        if self.validate {
            let info =
                naga::valid::Validator::new(naga::valid::ValidationFlags::all(), self.capabilities)
                    .validate(&naga_module);
            match info {
                Ok(_) => Ok(naga_module),
                Err(e) => {
                    let original_span = e.spans().last();
                    let err_source = match original_span.and_then(|(span, _)| span.to_range()) {
                        Some(rng) => {
                            let module_index = rng.start >> SPAN_SHIFT;
                            match module_index {
                                0 => ErrSource::Constructing {
                                    path: file_path.to_owned(),
                                    source: definition.substituted_source,
                                    offset: composable.start_offset,
                                },
                                _ => {
                                    let module_name =
                                        self.module_index.get(&module_index).unwrap().clone();
                                    let offset = self
                                        .module_sets
                                        .get(&module_name)
                                        .unwrap()
                                        .get_module(&shader_defs)
                                        .unwrap()
                                        .start_offset;
                                    ErrSource::Module(module_name, offset)
                                }
                            }
                        }
                        None => ErrSource::Constructing {
                            path: file_path.to_owned(),
                            source: modified_source,
                            offset: composable.start_offset,
                        },
                    };

                    Err(ComposerError {
                        inner: ComposerErrorInner::ShaderValidationError(e),
                        source: err_source,
                    })
                }
            }
        } else {
            Ok(naga_module)
        }
    }
}

static COMPOSER: once_cell::sync::Lazy<Composer> = once_cell::sync::Lazy::new(Composer::default);

/// Get module name and all required imports (ignoring shader_defs) from a shader string
pub fn get_preprocessor_data(source: &str) -> (Option<String>, Vec<ImportDefinition>) {
    let (name, imports) = COMPOSER.get_preprocessor_data(source);
    (
        name,
        imports
            .into_iter()
            .map(|import_with_offset| import_with_offset.definition)
            .collect(),
    )
}
