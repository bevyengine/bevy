use indexmap::IndexMap;
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
use naga::EntryPoint;
use regex::Regex;
use std::collections::{hash_map::Entry, BTreeMap, HashMap, HashSet};
use tracing::{debug, trace};

use crate::{
    compose::preprocess::{PreprocessOutput, PreprocessorMetaData},
    derive::DerivedModule,
    redirect::Redirector,
};

pub use self::error::{ComposerError, ComposerErrorInner, ErrSource};
use self::preprocess::Preprocessor;

pub mod comment_strip_iter;
pub mod error;
pub mod parse_imports;
pub mod preprocess;
mod test;
pub mod tokenizer;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum ShaderLanguage {
    #[default]
    Wgsl,
    #[cfg(feature = "glsl")]
    Glsl,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Default)]
pub enum ShaderType {
    #[default]
    Wgsl,
    #[cfg(feature = "glsl")]
    GlslVertex,
    #[cfg(feature = "glsl")]
    GlslFragment,
}

impl From<ShaderType> for ShaderLanguage {
    fn from(ty: ShaderType) -> Self {
        match ty {
            ShaderType::Wgsl => ShaderLanguage::Wgsl,
            #[cfg(feature = "glsl")]
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
    pub override_functions: IndexMap<String, Vec<String>>,
    // naga module, built against headers for any imports
    module_ir: naga::Module,
    // headers in different shader languages, used for building modules/shaders that import this module
    // headers contain types, constants, global vars and empty function definitions -
    // just enough to convert source strings that want to import this module into naga IR
    // headers: HashMap<ShaderLanguage, String>,
    header_ir: naga::Module,
    // character offset of the start of the owned module string
    start_offset: usize,
}

// data used to build a ComposableModule
#[derive(Debug)]
pub struct ComposableModuleDefinition {
    pub name: String,
    // shader text (with auto bindings replaced - we do this on module add as we only want to do it once to avoid burning slots)
    pub sanitized_source: String,
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
    // preprocessor meta data
    // metadata: PreprocessorMetaData,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportDefinition {
    pub import: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportDefWithOffset {
    definition: ImportDefinition,
    offset: usize,
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
    preprocessor: Preprocessor,
    check_decoration_regex: Regex,
    undecorate_regex: Regex,
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
            preprocessor: Preprocessor::default(),
            check_decoration_regex: Regex::new(
                format!(
                    "({}|{})",
                    regex_syntax::escape(DECORATION_PRE),
                    regex_syntax::escape(DECORATION_OVERRIDE_PRE)
                )
                .as_str(),
            )
            .unwrap(),
            undecorate_regex: Regex::new(
                format!(
                    r"(\x1B\[\d+\w)?([\w\d_]+){}([A-Z0-9]*){}",
                    regex_syntax::escape(DECORATION_PRE),
                    regex_syntax::escape(DECORATION_POST)
                )
                .as_str(),
            )
            .unwrap(),
            virtual_fn_regex: Regex::new(
                r"(?P<lead>[\s]*virtual\s+fn\s+)(?P<function>[^\s]+)(?P<trail>\s*)\(",
            )
            .unwrap(),
            override_fn_regex: Regex::new(
                format!(
                    r"(override\s+fn\s+)([^\s]+){}([\w\d]+){}(\s*)\(",
                    regex_syntax::escape(DECORATION_PRE),
                    regex_syntax::escape(DECORATION_POST)
                )
                .as_str(),
            )
            .unwrap(),
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

const DECORATION_PRE: &str = "X_naga_oil_mod_X";
const DECORATION_POST: &str = "X";

// must be same length as DECORATION_PRE for spans to work
const DECORATION_OVERRIDE_PRE: &str = "X_naga_oil_vrt_X";

struct IrBuildResult {
    module: naga::Module,
    start_offset: usize,
    override_functions: IndexMap<String, Vec<String>>,
}

impl Composer {
    pub fn decorated_name(module_name: Option<&str>, item_name: &str) -> String {
        match module_name {
            Some(module_name) => format!("{}{}", item_name, Self::decorate(module_name)),
            None => item_name.to_owned(),
        }
    }

    fn decorate(module: &str) -> String {
        let encoded = data_encoding::BASE32_NOPAD.encode(module.as_bytes());
        format!("{DECORATION_PRE}{encoded}{DECORATION_POST}")
    }

    fn decode(from: &str) -> String {
        String::from_utf8(data_encoding::BASE32_NOPAD.decode(from.as_bytes()).unwrap()).unwrap()
    }

    fn undecorate(&self, string: &str) -> String {
        let undecor = self
            .undecorate_regex
            .replace_all(string, |caps: &regex::Captures| {
                format!(
                    "{}{}::{}",
                    caps.get(1).map(|cc| cc.as_str()).unwrap_or(""),
                    Self::decode(caps.get(3).unwrap().as_str()),
                    caps.get(2).unwrap().as_str()
                )
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

    fn sanitize_and_set_auto_bindings(&mut self, source: &str) -> String {
        let mut substituted_source = source.replace("\r\n", "\n").replace('\r', "\n");
        if !substituted_source.ends_with('\n') {
            substituted_source.push('\n');
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

        substituted_source.into_owned()
    }

    fn naga_to_string(
        &self,
        naga_module: &mut naga::Module,
        language: ShaderLanguage,
        #[allow(unused)] header_for: &str, // Only used when GLSL is enabled
    ) -> Result<String, ComposerErrorInner> {
        // TODO: cache headers again
        let info =
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), self.capabilities)
                .validate(naga_module)
                .map_err(ComposerErrorInner::HeaderValidationError)?;

        match language {
            ShaderLanguage::Wgsl => naga::back::wgsl::write_string(
                naga_module,
                &info,
                naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
            )
            .map_err(ComposerErrorInner::WgslBackError),
            #[cfg(feature = "glsl")]
            ShaderLanguage::Glsl => {
                let vec4 = naga_module.types.insert(
                    naga::Type {
                        name: None,
                        inner: naga::TypeInner::Vector {
                            size: naga::VectorSize::Quad,
                            scalar: naga::Scalar::F32,
                        },
                    },
                    naga::Span::UNDEFINED,
                );
                // add a dummy entry point for glsl headers
                let dummy_entry_point = "dummy_module_entry_point".to_owned();
                let func = naga::Function {
                    name: Some(dummy_entry_point.clone()),
                    arguments: Default::default(),
                    result: Some(naga::FunctionResult {
                        ty: vec4,
                        binding: Some(naga::Binding::BuiltIn(naga::BuiltIn::Position {
                            invariant: false,
                        })),
                    }),
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

                naga_module.entry_points.push(ep);

                let info = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    self.capabilities,
                )
                .validate(naga_module)
                .map_err(ComposerErrorInner::HeaderValidationError)?;

                let mut string = String::new();
                let options = naga::back::glsl::Options {
                    version: naga::back::glsl::Version::Desktop(450),
                    writer_flags: naga::back::glsl::WriterFlags::INCLUDE_UNUSED_ITEMS,
                    ..Default::default()
                };
                let pipeline_options = naga::back::glsl::PipelineOptions {
                    shader_stage: naga::ShaderStage::Vertex,
                    entry_point: dummy_entry_point,
                    multiview: None,
                };
                let mut writer = naga::back::glsl::Writer::new(
                    &mut string,
                    naga_module,
                    &info,
                    &options,
                    &pipeline_options,
                    naga::proc::BoundsCheckPolicies::default(),
                )
                .map_err(ComposerErrorInner::GlslBackError)?;

                writer.write().map_err(ComposerErrorInner::GlslBackError)?;

                // strip version decl and main() impl
                let lines: Vec<_> = string.lines().collect();
                let string = lines[1..lines.len() - 3].join("\n");
                trace!("glsl header for {}:\n\"\n{:?}\n\"", header_for, string);

                Ok(string)
            }
        }
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
            #[cfg(feature = "glsl")]
            ShaderLanguage::Glsl => String::from("#version 450\n"),
        };

        let mut override_functions: IndexMap<String, Vec<String>> = IndexMap::default();
        let mut added_imports: HashSet<String> = HashSet::new();
        let mut header_module = DerivedModule::default();

        for import in imports {
            if added_imports.contains(&import.import) {
                continue;
            }
            // add to header module
            self.add_import(
                &mut header_module,
                import,
                shader_defs,
                true,
                &mut added_imports,
            );

            // // we must have ensured these exist with Composer::ensure_imports()
            trace!("looking for {}", import.import);
            let import_module_set = self.module_sets.get(&import.import).unwrap();
            trace!("with defs {:?}", shader_defs);
            let module = import_module_set.get_module(shader_defs).unwrap();
            trace!("ok");

            // gather overrides
            if !module.override_functions.is_empty() {
                for (original, replacements) in &module.override_functions {
                    match override_functions.entry(original.clone()) {
                        indexmap::map::Entry::Occupied(o) => {
                            let existing = o.into_mut();
                            let new_replacements: Vec<_> = replacements
                                .iter()
                                .filter(|rep| !existing.contains(rep))
                                .cloned()
                                .collect();
                            existing.extend(new_replacements);
                        }
                        indexmap::map::Entry::Vacant(v) => {
                            v.insert(replacements.clone());
                        }
                    }
                }
            }
        }

        let composed_header = self
            .naga_to_string(&mut header_module.into(), language, name)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Module {
                    name: name.to_owned(),
                    offset: 0,
                    defs: shader_defs.clone(),
                },
            })?;
        module_string.push_str(&composed_header);

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
                    source: ErrSource::Module {
                        name: name.to_owned(),
                        offset: start_offset,
                        defs: shader_defs.clone(),
                    },
                }
            })?,
            #[cfg(feature = "glsl")]
            ShaderLanguage::Glsl => naga::front::glsl::Frontend::default()
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
                        source: ErrSource::Module {
                            name: name.to_owned(),
                            offset: start_offset,
                            defs: shader_defs.clone(),
                        },
                    }
                })?,
        };

        Ok(IrBuildResult {
            module,
            start_offset,
            override_functions,
        })
    }

    // check that identifiers exported by a module do not get modified in string export
    fn validate_identifiers(
        source_ir: &naga::Module,
        lang: ShaderLanguage,
        header: &str,
        module_decoration: &str,
        owned_types: &HashSet<String>,
    ) -> Result<(), ComposerErrorInner> {
        // TODO: remove this once glsl front support is complete
        #[cfg(feature = "glsl")]
        if lang == ShaderLanguage::Glsl {
            return Ok(());
        }

        let recompiled = match lang {
            ShaderLanguage::Wgsl => naga::front::wgsl::parse_str(header).unwrap(),
            #[cfg(feature = "glsl")]
            ShaderLanguage::Glsl => naga::front::glsl::Frontend::default()
                .parse(
                    &naga::front::glsl::Options {
                        stage: naga::ShaderStage::Vertex,
                        defines: Default::default(),
                    },
                    &format!("{}\n{}", header, "void main() {}"),
                )
                .map_err(|e| {
                    debug!("full err'd source file: \n---\n{header}\n---");
                    ComposerErrorInner::GlslParseError(e)
                })?,
        };

        let recompiled_types: IndexMap<_, _> = recompiled
            .types
            .iter()
            .flat_map(|(h, ty)| ty.name.as_deref().map(|name| (name, h)))
            .collect();
        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                let decorated_type_name = format!("{name}{module_decoration}");
                if !owned_types.contains(&decorated_type_name) {
                    continue;
                }
                match recompiled_types.get(decorated_type_name.as_str()) {
                    Some(recompiled_h) => {
                        if let naga::TypeInner::Struct { members, .. } = &ty.inner {
                            let recompiled_ty = recompiled.types.get_handle(*recompiled_h).unwrap();
                            let naga::TypeInner::Struct {
                                members: recompiled_members,
                                ..
                            } = &recompiled_ty.inner
                            else {
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
            .filter(|name| name.ends_with(module_decoration))
            .collect();
        for (h, c) in source_ir.constants.iter() {
            if let Some(name) = &c.name {
                if name.ends_with(module_decoration) && !recompiled_consts.contains(name.as_str()) {
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
            .filter(|name| name.ends_with(module_decoration))
            .collect();
        for (h, gv) in source_ir.global_variables.iter() {
            if let Some(name) = &gv.name {
                if name.ends_with(module_decoration) && !recompiled_globals.contains(name.as_str())
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
            .filter(|name| name.ends_with(module_decoration))
            .collect();
        for (h, f) in source_ir.functions.iter() {
            if let Some(name) = &f.name {
                if name.ends_with(module_decoration) && !recompiled_fns.contains(name.as_str()) {
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
    #[allow(clippy::too_many_arguments)]
    fn create_composable_module(
        &mut self,
        module_definition: &ComposableModuleDefinition,
        module_decoration: String,
        shader_defs: &HashMap<String, ShaderDefValue>,
        create_headers: bool,
        demote_entrypoints: bool,
        source: &str,
        imports: Vec<ImportDefWithOffset>,
    ) -> Result<ComposableModule, ComposerError> {
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
            .replace_all(source, |cap: &regex::Captures| {
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
        let mut local_override_functions: IndexMap<String, String> = Default::default();

        #[cfg(not(feature = "override_any"))]
        let mut override_error = None;

        let source =
            self.override_fn_regex
                .replace_all(&source, |cap: &regex::Captures| {
                    let target_module = cap.get(3).unwrap().as_str().to_owned();
                    let target_function = cap.get(2).unwrap().as_str().to_owned();

                    #[cfg(not(feature = "override_any"))]
                    {
                        let wrap_err = |inner: ComposerErrorInner| -> ComposerError {
                            ComposerError {
                                inner,
                                source: ErrSource::Module {
                                    name: module_definition.name.to_owned(),
                                    offset: 0,
                                    defs: shader_defs.clone(),
                                },
                            }
                        };

                        // ensure overrides are applied to virtual functions
                        let raw_module_name = Self::decode(&target_module);
                        let module_set = self.module_sets.get(&raw_module_name);

                        match module_set {
                            None => {
                                // TODO this should be unreachable?
                                let pos = cap.get(3).unwrap().start();
                                override_error = Some(wrap_err(
                                    ComposerErrorInner::ImportNotFound(raw_module_name, pos),
                                ));
                            }
                            Some(module_set) => {
                                let module = module_set.get_module(shader_defs).unwrap();
                                if !module.virtual_functions.contains(&target_function) {
                                    let pos = cap.get(2).unwrap().start();
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
                        target_function.as_str(),
                        DECORATION_PRE,
                        target_module.as_str(),
                        DECORATION_POST,
                    );
                    let rename = format!(
                        "{}{}{}{}",
                        target_function.as_str(),
                        DECORATION_OVERRIDE_PRE,
                        target_module.as_str(),
                        DECORATION_POST,
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
                source: ErrSource::Module {
                    name: module_definition.name.to_owned(),
                    offset: start_offset,
                    defs: shader_defs.clone(),
                },
            }
        };

        // add our local override to the total set of overrides for the given function
        for (rename, base_name) in &local_override_functions {
            override_functions
                .entry(base_name.clone())
                .or_default()
                .push(format!("{rename}{module_decoration}"));
        }

        // rename and record owned items (except types which can't be mutably accessed)
        let mut owned_constants = IndexMap::new();
        for (h, c) in source_ir.constants.iter_mut() {
            if let Some(name) = c.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{name}{module_decoration}");
                    owned_constants.insert(name.clone(), h);
                }
            }
        }

        let mut owned_vars = IndexMap::new();
        for (h, gv) in source_ir.global_variables.iter_mut() {
            if let Some(name) = gv.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{name}{module_decoration}");

                    owned_vars.insert(name.clone(), h);
                }
            }
        }

        let mut owned_functions = IndexMap::new();
        for (h_f, f) in source_ir.functions.iter_mut() {
            if let Some(name) = f.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{name}{module_decoration}");

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
                    ep.function.name.as_deref().unwrap_or("main"),
                    module_decoration,
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
                // we need to exclude autogenerated struct names, i.e. those that begin with "__"
                // "__" is a reserved prefix for naga so user variables cannot use it.
                if !name.contains(DECORATION_PRE) && !name.starts_with("__") {
                    let name = format!("{name}{module_decoration}");
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

        let module_ir = module_builder.into_module_with_entrypoints();
        let mut header_ir: naga::Module = header_builder.into();

        if self.validate && create_headers {
            // check that identifiers haven't been renamed
            #[allow(clippy::single_element_loop)]
            for language in [
                ShaderLanguage::Wgsl,
                #[cfg(feature = "glsl")]
                ShaderLanguage::Glsl,
            ] {
                let header = self
                    .naga_to_string(&mut header_ir, language, &module_definition.name)
                    .map_err(wrap_err)?;
                Self::validate_identifiers(
                    &source_ir,
                    language,
                    &header,
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
            header_ir,
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
        header: bool,
    ) {
        let items: Option<HashSet<String>> = items.map(|items| {
            items
                .iter()
                .map(|item| format!("{}{}", item, composable.decorated_name))
                .collect()
        });
        let items = items.as_ref();

        let source_ir = match header {
            true => &composable.header_ir,
            false => &composable.module_ir,
        };

        derived.set_shader_source(source_ir, span_offset);

        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                if composable.owned_types.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_type(&h);
                }
            }
        }

        for (h, c) in source_ir.constants.iter() {
            if let Some(name) = &c.name {
                if composable.owned_constants.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_const(&h);
                }
            }
        }

        for (h, v) in source_ir.global_variables.iter() {
            if let Some(name) = &v.name {
                if composable.owned_vars.contains(name)
                    && items.map_or(true, |items| items.contains(name))
                {
                    derived.import_global(&h);
                }
            }
        }

        for (h_f, f) in source_ir.functions.iter() {
            if let Some(name) = &f.name {
                if composable.owned_functions.contains(name)
                    && (items.map_or(true, |items| items.contains(name))
                        || composable
                            .override_functions
                            .values()
                            .any(|v| v.contains(name)))
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
        header: bool,
        already_added: &mut HashSet<String>,
    ) {
        if already_added.contains(&import.import) {
            trace!("skipping {}, already added", import.import);
            return;
        }

        let import_module_set = self.module_sets.get(&import.import).unwrap();
        let module = import_module_set.get_module(shader_defs).unwrap();

        for import in &module.imports {
            self.add_import(derived, import, shader_defs, header, already_added);
        }

        Self::add_composable_data(
            derived,
            module,
            Some(&import.items),
            import_module_set.module_index << SPAN_SHIFT,
            header,
        );
    }

    fn ensure_import(
        &mut self,
        module_set: &ComposableModuleDefinition,
        shader_defs: &HashMap<String, ShaderDefValue>,
    ) -> Result<ComposableModule, ComposerError> {
        let PreprocessOutput {
            preprocessed_source,
            imports,
        } = self
            .preprocessor
            .preprocess(&module_set.sanitized_source, shader_defs, self.validate)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Module {
                    name: module_set.name.to_owned(),
                    offset: 0,
                    defs: shader_defs.clone(),
                },
            })?;

        self.ensure_imports(imports.iter().map(|import| &import.definition), shader_defs)?;
        self.ensure_imports(&module_set.additional_imports, shader_defs)?;

        self.create_composable_module(
            module_set,
            Self::decorate(&module_set.name),
            shader_defs,
            true,
            true,
            &preprocessed_source,
            imports,
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

        let substituted_source = self.sanitize_and_set_auto_bindings(source);

        let PreprocessorMetaData {
            name: module_name,
            mut imports,
            mut effective_defs,
            ..
        } = self
            .preprocessor
            .get_preprocessor_metadata(&substituted_source, false)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            })?;
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
                        source: substituted_source.to_owned(),
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

        // remove defs that are already specified through our imports
        effective_defs.retain(|name| !shader_defs.contains_key(name));

        // can't gracefully report errors for more modules. perhaps this should be a warning
        assert!((self.module_sets.len() as u32) < u32::MAX >> SPAN_SHIFT);
        let module_index = self.module_sets.len() + 1;

        let module_set = ComposableModuleDefinition {
            name: module_name.clone(),
            sanitized_source: substituted_source,
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

        let sanitized_source = self.sanitize_and_set_auto_bindings(source);

        let PreprocessorMetaData {
            name,
            defines,
            imports,
            ..
        } = self
            .preprocessor
            .get_preprocessor_metadata(&sanitized_source, true)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: sanitized_source.to_owned(),
                    offset: 0,
                },
            })?;
        shader_defs.extend(defines);

        let name = name.unwrap_or_default();

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
                                    source: sanitized_source.to_owned(),
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
                        source: sanitized_source,
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
            sanitized_source: sanitized_source.clone(),
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

        let PreprocessOutput {
            preprocessed_source,
            imports,
        } = self
            .preprocessor
            .preprocess(&sanitized_source, &shader_defs, self.validate)
            .map_err(|inner| ComposerError {
                inner,
                source: ErrSource::Constructing {
                    path: file_path.to_owned(),
                    source: sanitized_source,
                    offset: 0,
                },
            })?;

        let composable = self
            .create_composable_module(
                &definition,
                String::from(""),
                &shader_defs,
                false,
                false,
                &preprocessed_source,
                imports,
            )
            .map_err(|e| ComposerError {
                inner: e.inner,
                source: ErrSource::Constructing {
                    path: definition.file_path.to_owned(),
                    source: preprocessed_source.clone(),
                    offset: e.source.offset(),
                },
            })?;

        let mut derived = DerivedModule::default();

        let mut already_added = Default::default();
        for import in &composable.imports {
            self.add_import(
                &mut derived,
                import,
                &shader_defs,
                false,
                &mut already_added,
            );
        }

        Self::add_composable_data(&mut derived, &composable, None, 0, false);

        let stage = match shader_type {
            #[cfg(feature = "glsl")]
            ShaderType::GlslVertex => Some(naga::ShaderStage::Vertex),
            #[cfg(feature = "glsl")]
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
                                source: preprocessed_source.clone(),
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
                    source: preprocessed_source.clone(),
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
                                    source: preprocessed_source.clone(),
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
                                    ErrSource::Module {
                                        name: module_name,
                                        offset,
                                        defs: shader_defs.clone(),
                                    }
                                }
                            }
                        }
                        None => ErrSource::Constructing {
                            path: file_path.to_owned(),
                            source: preprocessed_source.clone(),
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

static PREPROCESSOR: once_cell::sync::Lazy<Preprocessor> =
    once_cell::sync::Lazy::new(Preprocessor::default);

/// Get module name and all required imports (ignoring shader_defs) from a shader string
pub fn get_preprocessor_data(
    source: &str,
) -> (
    Option<String>,
    Vec<ImportDefinition>,
    HashMap<String, ShaderDefValue>,
) {
    if let Ok(PreprocessorMetaData {
        name,
        imports,
        defines,
        ..
    }) = PREPROCESSOR.get_preprocessor_metadata(source, true)
    {
        (
            name,
            imports
                .into_iter()
                .map(|import_with_offset| import_with_offset.definition)
                .collect(),
            defines,
        )
    } else {
        // if errors occur we return nothing; the actual error will be displayed when the caller attempts to use the shader
        Default::default()
    }
}
