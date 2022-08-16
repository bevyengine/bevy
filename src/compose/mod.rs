use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term,
};
use naga::EntryPoint;
use regex::Regex;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    iter::FusedIterator,
    ops::Range,
};
use thiserror::Error;
use tracing::debug;

use crate::derive::DerivedModule;

mod test;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ShaderLanguage {
    Wgsl,
    Glsl,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum ShaderType {
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

// a module built with a specific set of shader_defs
#[derive(Default, Debug)]
pub struct ComposableModule {
    pub imports: Vec<String>,
    // types exported
    pub owned_types: HashSet<String>,
    // constants exported
    pub owned_constants: HashSet<String>,
    // vars exported
    pub owned_vars: HashSet<String>,
    // functions exported
    pub owned_functions: HashSet<String>,
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
    pub path: String,
    // list of shader_defs that can affect this module
    effective_defs: Vec<String>,
    // full list of possible imports (regardless of shader_def configuration)
    all_imports: HashSet<String>,
    // built composable modules for a given set of shader defs
    modules: HashMap<Vec<String>, ComposableModule>,
    // used in spans when this module is included
    module_index: usize,
}

impl ComposableModuleDefinition {
    fn get(&self, shader_defs: &HashSet<String>) -> Option<&ComposableModule> {
        let mut effective_defs: Vec<_> = self
            .effective_defs
            .iter()
            .filter(|&def| shader_defs.contains(def))
            .cloned()
            .collect();
        effective_defs.sort();
        self.modules.get(&effective_defs)
    }

    fn insert(
        &mut self,
        shader_defs: &HashSet<String>,
        module: ComposableModule,
    ) -> &ComposableModule {
        let mut effective_defs: Vec<_> = self
            .effective_defs
            .iter()
            .filter(|&def| shader_defs.contains(def))
            .cloned()
            .collect();
        effective_defs.sort();
        self.modules.insert(effective_defs.clone(), module);
        self.modules.get(&effective_defs).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct ImportDefinition {
    pub import: String,
    pub as_name: String,
    pub offset: usize,
}

#[derive(Debug)]
enum ErrSource {
    Composed(String, HashSet<String>),
    Constructing {
        path: String,
        source: String,
        offset: usize,
    },
}

impl ErrSource {
    fn path<'a>(&'a self, composer: &'a Composer) -> &'a String {
        match self {
            ErrSource::Composed(c, _) => &composer.module_sets.get(c).unwrap().path,
            ErrSource::Constructing { path, .. } => path,
        }
    }

    fn source<'a>(&'a self, composer: &'a Composer) -> &'a String {
        match self {
            ErrSource::Composed(c, _) => &composer.module_sets.get(c).unwrap().substituted_source,
            ErrSource::Constructing { source, .. } => source,
        }
    }

    fn offset(&self, composer: &Composer) -> usize {
        match self {
            ErrSource::Composed(c, defs) => {
                composer
                    .module_sets
                    .get(c)
                    .unwrap()
                    .get(defs)
                    .unwrap()
                    .start_offset
            }
            ErrSource::Constructing { offset, .. } => *offset,
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
    #[error("Attempted to add a module with no #define_import_path")]
    NoModuleName,
    #[error("source contains internal decoration string, results probably won't be what you expect. if you have a legitimate reason to do this please file a report")]
    DecorationInSource(Range<usize>),
    #[error("naga oil only supports glsl 440 and 450")]
    GlslInvalidVersion(usize),
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
        let source_offset = self.source.offset(composer);

        println!("emit error: {:#?}", self);

        debug!("source:\n~{}~", source);
        debug!("source offset: {}", source_offset);

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
                        debug!(
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
                vec![format!("missing import '{}'", msg)],
            ),
            ComposerErrorInner::WgslParseError(e) => (
                e.labels()
                    .map(|(range, msg)| Label::primary((), map_span(range)).with_message(msg))
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
            | ComposerErrorInner::GlslInvalidVersion(pos) => {
                (vec![Label::primary((), *pos..*pos)], vec![])
            }
            ComposerErrorInner::WgslBackError(e) => {
                return format!("{}: wgsl back error: {}", path, e.to_string());
            }
            ComposerErrorInner::GlslBackError(e) => {
                return format!("{}: glsl back error: {}", path, e.to_string());
            }
            ComposerErrorInner::NoModuleName => {
                return format!(
                    "{}: no #define_import_path declaration found in composable module",
                    path
                );
            }
        };

        let diagnostic = Diagnostic::error()
            .with_message(self.inner.to_string())
            .with_labels(labels)
            .with_notes(notes);

        term::emit(&mut writer, &config, &files, &diagnostic).expect("cannot write error");

        let msg = writer.into_inner();
        let msg = String::from_utf8_lossy(&msg);

        return msg.to_string();
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
    decoration_regex: Regex,
    undecorate_regex: Regex,
    version_regex: Regex,
    ifdef_regex: Regex,
    ifndef_regex: Regex,
    else_regex: Regex,
    endif_regex: Regex,
    import_custom_path_as_regex: Regex,
    import_custom_path_regex: Regex,
    define_import_path_regex: Regex,
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
            decoration_regex: Regex::new(regex_syntax::escape(DECORATION_PRE).as_str()).unwrap(),
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
            else_regex: Regex::new(r"^\s*#\s*else").unwrap(),
            endif_regex: Regex::new(r"^\s*#\s*endif").unwrap(),
            import_custom_path_as_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)\s+as\s+([^\s]+)")
                .unwrap(),
            import_custom_path_regex: Regex::new(r"^\s*#\s*import\s+([^\s]+)").unwrap(),
            define_import_path_regex: Regex::new(r"^\s*#\s*define_import_path\s+([^\s]+)").unwrap(),
        }
    }
}

const DECORATION_PRE: &str = "_naga_oil__mod__";
const DECORATION_POST: &str = "__member__";

impl Composer {
    fn decorate(as_name: &str) -> String {
        let as_name = data_encoding::BASE32_NOPAD.encode(as_name.as_bytes());
        format!("{}{}{}", DECORATION_PRE, as_name, DECORATION_POST)
    }

    fn undecorate(&self, string: &str) -> String {
        self.undecorate_regex
            .replace_all(string, |caps: &regex::Captures| {
                format!(
                    "{}::",
                    String::from_utf8(
                        data_encoding::BASE32_NOPAD
                            .decode(caps.get(1).unwrap().as_str().as_bytes())
                            .unwrap()
                    )
                    .unwrap()
                )
            })
            .to_string()
    }

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

    fn sanitize_and_substitute_shader_string(source: &str, imports: &[ImportDefinition]) -> String {
        let mut substituted_source = source.replace("\r\n", "\n").replace("\r", "\n");
        if substituted_source.chars().last().unwrap() != '\n' {
            substituted_source.push_str("\n");
        }
        for ImportDefinition {
            import, as_name, ..
        } in imports.iter()
        {
            substituted_source = substituted_source
                .replace(format!("{}::", as_name).as_str(), &Self::decorate(import));
        }
        substituted_source
    }

    // recursively add header strings to the input string for each imported module
    // requires that imports are already built
    fn add_header_strings<'a>(
        &'a self,
        lang: ShaderLanguage,
        module_string: &mut String,
        imports: &'a Vec<String>,
        shader_defs: &HashSet<String>,
    ) {
        for import in imports {
            // // we must have ensured these exist with Composer::ensure_imports()
            let import_module_set = self.module_sets.get(import).unwrap();
            let module = import_module_set.get(shader_defs).unwrap();
            module_string.push_str(module.headers.get(&lang).unwrap().as_str());
        }
    }

    // build naga module for a given shader_def configuration. builds a minimal self-contained module built against headers for imports
    fn create_module_ir(
        &mut self,
        name: &str,
        source: String,
        path: &str,
        language: ShaderLanguage,
        imports: &[ImportDefinition],
        shader_defs: &HashSet<String>,
    ) -> Result<(naga::Module, usize, String), ComposerError> {
        let mut module_string = match language {
            ShaderLanguage::Wgsl => String::new(),
            ShaderLanguage::Glsl => String::from("#version 450\n"),
        };

        self.add_header_strings(
            language,
            &mut module_string,
            &imports.iter().map(|def| &def.import).cloned().collect(),
            shader_defs,
        );

        let start_offset = module_string.len();

        module_string.push_str(&source);

        debug!(
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
                    source: ErrSource::Constructing {
                        path: path.to_owned(),
                        source,
                        offset: start_offset,
                    },
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
                        source: ErrSource::Constructing {
                            path: path.to_owned(),
                            source,
                            offset: start_offset,
                        },
                    }
                })?,
        };

        Ok((module, start_offset, module_string))
    }

    // process #if(n)def / #else / #endif preprocessor directives,
    // strip module name and imports
    // also strip "#version xxx"
    fn preprocess_defs(
        &self,
        _mod_name: &str,
        shader_str: &str,
        path: &str,
        shader_defs: &HashSet<String>,
        validate_len: bool,
    ) -> Result<(Option<String>, String, Vec<ImportDefinition>), ComposerError> {
        let mut imports = Vec::new();
        let mut scopes = vec![true];
        let mut final_string = String::new();
        let mut name = None;
        let mut offset = 0;

        let len = shader_str.len();

        // this code broadly stolen from bevy_render::ShaderProcessor
        for line in shader_str.lines() {
            let mut output = false;
            if let Some(cap) = self.version_regex.captures(line) {
                let v = cap.get(1).unwrap().as_str();
                if v != "440" && v != "450" {
                    return Err(ComposerError {
                        inner: ComposerErrorInner::GlslInvalidVersion(offset),
                        source: ErrSource::Constructing {
                            path: path.to_owned(),
                            source: shader_str.to_owned(),
                            offset: 0,
                        },
                    });
                }
            } else if let Some(cap) = self.ifdef_regex.captures(line) {
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
                    return Err(ComposerError {
                        inner: ComposerErrorInner::TooManyEndIfs(offset),
                        source: ErrSource::Constructing {
                            path: path.to_owned(),
                            source: shader_str.to_owned(),
                            offset: 0,
                        },
                    });
                }
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            } else if *scopes.last().unwrap() {
                if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                    imports.push(ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: cap.get(2).unwrap().as_str().to_string(),
                        offset,
                    });
                } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                    imports.push(ImportDefinition {
                        import: cap.get(1).unwrap().as_str().to_string(),
                        as_name: cap.get(1).unwrap().as_str().to_string(),
                        offset,
                    });
                } else {
                    final_string.push_str(line);
                    output = true;
                }
            }

            if !output {
                final_string.extend(std::iter::repeat(" ").take(line.len()));
            }
            final_string.push_str("\n");
            offset += line.chars().count() + 1;
        }

        if scopes.len() != 1 {
            return Err(ComposerError {
                inner: ComposerErrorInner::NotEnoughEndIfs(offset),
                source: ErrSource::Constructing {
                    path: path.to_owned(),
                    source: shader_str.to_owned(),
                    offset: 0,
                },
            });
        }

        let revised_len = final_string.len();

        if validate_len {
            debug!("1~{}~\n2~{}~\n", shader_str, final_string);
            assert_eq!(len, revised_len);
        }

        // sort imports by decreasing length so we don't accidentally replace substrings of a longer import
        imports.sort_by_key(|import| usize::MAX - import.as_name.len());

        Ok((name, final_string, imports))
    }

    // extract module name and imports
    fn get_preprocessor_data(&self, shader_str: &str) -> (Option<String>, Vec<ImportDefinition>) {
        let mut imports = Vec::new();
        let mut name = None;
        let mut offset = 0;
        for line in shader_str.lines() {
            if let Some(cap) = self.import_custom_path_as_regex.captures(line) {
                imports.push(ImportDefinition {
                    import: cap.get(1).unwrap().as_str().to_string(),
                    as_name: cap.get(2).unwrap().as_str().to_string(),
                    offset,
                });
            } else if let Some(cap) = self.import_custom_path_regex.captures(line) {
                imports.push(ImportDefinition {
                    import: cap.get(1).unwrap().as_str().to_string(),
                    as_name: cap.get(1).unwrap().as_str().to_string(),
                    offset,
                });
            } else if let Some(cap) = self.define_import_path_regex.captures(line) {
                name = Some(cap.get(1).unwrap().as_str().to_string());
            }

            offset += line.chars().count() + 1;
        }

        // sort imports by decreasing length so we don't accidentally replace substrings of a longer import
        imports.sort_by_key(|import| usize::MAX - import.as_name.len());

        (name, imports)
    }

    // build a ComposableModule from a ComposableModuleDefinition, for a given set of shader defs
    // - build the naga IR (against headers)
    // - record any types/vars/constants/functions that are defined within this module
    // - build headers for each supported language
    fn create_composable_module(
        &mut self,
        module_name: &str,
        path: &str,
        module_decoration: &str,
        unprocessed_source: &str,
        language: ShaderLanguage,
        shader_defs: &HashSet<String>,
        create_headers: bool,
    ) -> Result<ComposableModule, ComposerError> {
        let (_, source, imports) =
            self.preprocess_defs(module_name, unprocessed_source, path, shader_defs, true)?;

        debug!(
            "create composable module {}: source len {}",
            module_name,
            source.len()
        );
        let (mut source_ir, start_offset, _) = self.create_module_ir(
            module_name,
            source.clone(),
            path,
            language,
            &imports,
            shader_defs,
        )?;

        // rename and record owned items (except types which can't be mutably accessed)
        let mut owned_constants = HashMap::new();
        for (h, c) in source_ir.constants.iter_mut() {
            if let Some(name) = c.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);
                    owned_constants.insert(name.clone(), h);
                }
            }
        }

        let mut owned_vars = HashMap::new();
        for (h, gv) in source_ir.global_variables.iter_mut() {
            if let Some(name) = gv.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);

                    owned_vars.insert(name.clone(), h);
                }
            }
        }

        let mut owned_functions = HashMap::new();
        for (h_f, f) in source_ir.functions.iter_mut() {
            if let Some(name) = f.name.as_mut() {
                if !name.contains(DECORATION_PRE) {
                    *name = format!("{}{}", module_decoration, name);

                    let header_function = naga::Function {
                        name: Some(name.clone()),
                        arguments: f.arguments.to_vec(),
                        result: f.result.clone(),
                        local_variables: Default::default(),
                        expressions: Default::default(),
                        named_expressions: Default::default(),
                        body: Default::default(),
                    };
                    owned_functions.insert(name.clone(), (h_f, header_function));
                }
            }
        }

        let mut module_builder = DerivedModule::default();
        let mut header_builder = DerivedModule::default();
        module_builder.set_shader_source(&source_ir, 0);
        header_builder.set_shader_source(&source_ir, 0);

        let mut owned_types = HashSet::new();
        for (h, ty) in source_ir.types.iter() {
            if let Some(name) = &ty.name {
                if !name.contains(DECORATION_PRE) {
                    let name = format!("{}{}", module_decoration, name);
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
        for (_, h) in owned_constants.iter() {
            header_builder.import_const(h);
            module_builder.import_const(h);
        }

        for (_, h) in owned_vars.iter() {
            header_builder.import_global(h);
            module_builder.import_global(h);
        }

        // only stubs of owned functions into the header
        for (_, (h_f, f)) in owned_functions.iter() {
            let span = source_ir.functions.get_span(*h_f);
            header_builder.import_function(f, span); // header stub function
        }
        // all functions into the module (note source_ir only contains stubs for imported functions)
        for (h_f, f) in source_ir.functions.iter() {
            let span = source_ir.functions.get_span(h_f);
            module_builder.import_function(f, span);
        }

        let mut header_ir: naga::Module = header_builder.into();

        let info =
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), self.capabilities)
                .validate(&header_ir);

        let info = match info {
            Ok(info) => info,
            Err(e) => {
                return Err(ComposerError {
                    inner: ComposerErrorInner::HeaderValidationError(e),
                    source: ErrSource::Constructing {
                        path: path.to_owned(),
                        source: unprocessed_source.to_owned(),
                        offset: start_offset,
                    },
                });
            }
        };

        let headers = if create_headers {
            [
                (
                    ShaderLanguage::Wgsl,
                    naga::back::wgsl::write_string(
                        &header_ir,
                        &info,
                        naga::back::wgsl::WriterFlags::EXPLICIT_TYPES,
                    )
                    .map_err(|e| ComposerError {
                        inner: ComposerErrorInner::WgslBackError(e),
                        source: ErrSource::Constructing {
                            path: path.to_owned(),
                            source: unprocessed_source.to_owned(),
                            offset: start_offset,
                        },
                    })?,
                ),
                (
                    // note this must come last as we add a dummy entry point
                    ShaderLanguage::Glsl,
                    {
                        // add a dummy entry point for glsl headers
                        let dummy_entry_point =
                            format!("{}dummy_module_entry_point", module_decoration);
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
                        .validate(&header_ir);

                        let info = match info {
                            Ok(info) => info,
                            Err(e) => {
                                return Err(ComposerError {
                                    inner: ComposerErrorInner::HeaderValidationError(e),
                                    source: ErrSource::Constructing {
                                        path: path.to_owned(),
                                        source: unprocessed_source.to_owned(),
                                        offset: start_offset,
                                    },
                                });
                            }
                        };

                        let mut string = String::new();
                        let options = naga::back::glsl::Options {
                            version: naga::back::glsl::Version::Desktop(450),
                            writer_flags: naga::back::glsl::WriterFlags::empty(),
                            binding_map: Default::default(),
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
                            source: ErrSource::Constructing {
                                path: path.to_owned(),
                                source: unprocessed_source.to_owned(),
                                offset: start_offset,
                            },
                        })?;
                        writer.write().map_err(|e| ComposerError {
                            inner: ComposerErrorInner::GlslBackError(e),
                            source: ErrSource::Constructing {
                                path: path.to_owned(),
                                source: unprocessed_source.to_owned(),
                                offset: start_offset,
                            },
                        })?;
                        // strip version decl and main() impl
                        let lines: Vec<_> = string.lines().collect();
                        let string = lines[1..lines.len() - 3].join("\n");
                        debug!("glsl header for {}:\n\"\n{:?}\n\"", module_name, string);
                        string
                    },
                ),
            ]
            .into()
        } else {
            Default::default()
        };

        let entry_points = source_ir
            .entry_points
            .iter()
            .map(|ep| EntryPoint {
                name: ep.name.clone(),
                stage: ep.stage,
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
                function: module_builder.localize_function(&ep.function),
            })
            .collect();

        let module_ir = naga::Module {
            entry_points,
            ..module_builder.into()
        };

        let composable_module = ComposableModule {
            imports: imports.into_iter().map(|def| def.import).collect(),
            owned_types,
            owned_constants: owned_constants.into_iter().map(|(n, _)| n).collect(),
            owned_vars: owned_vars.into_iter().map(|(n, _)| n).collect(),
            owned_functions: owned_functions.into_iter().map(|(n, _)| n).collect(),
            module_ir,
            headers,
            start_offset,
        };

        Ok(composable_module)
    }

    /// add a composable module to the composer.
    /// all modules imported by this module must already have been added
    pub fn add_composable_module(
        &mut self,
        source: &str,
        path: &str,
        language: ShaderLanguage,
    ) -> Result<&ComposableModuleDefinition, ComposerError> {
        // reject a module containing the DECORATION_PRE string
        if let Some(decor) = self.decoration_regex.find(source) {
            return Err(ComposerError {
                inner: ComposerErrorInner::DecorationInSource(decor.range()),
                source: ErrSource::Constructing {
                    path: path.to_owned(),
                    source: source.to_owned(),
                    offset: 0,
                },
            });
        }

        // use btreeset so the result is sorted
        let mut effective_defs = BTreeSet::new();

        let (module_name, imports) = self.get_preprocessor_data(&source);
        let substituted_source = Self::sanitize_and_substitute_shader_string(source, &imports);

        if module_name.is_none() {
            return Err(ComposerError {
                inner: ComposerErrorInner::NoModuleName,
                source: ErrSource::Constructing {
                    path: path.to_owned(),
                    source: substituted_source.clone(),
                    offset: 0,
                },
            });
        }

        let module_name = module_name.unwrap();

        for import in imports.iter() {
            // we require modules already added so that we can capture the shader_defs that may impact us by impacting our dependencies
            let module_set = self
                .module_sets
                .get(&import.import)
                .ok_or_else(|| ComposerError {
                    inner: ComposerErrorInner::ImportNotFound(import.import.clone(), import.offset),
                    source: ErrSource::Constructing {
                        path: path.to_owned(),
                        source: substituted_source.clone(),
                        offset: 0,
                    },
                })?;
            effective_defs.extend(module_set.effective_defs.iter().map(String::as_str));
        }

        // record our explicit effective shader_defs
        for line in source.lines() {
            if let Some(cap) = self.ifdef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str());
            }
            if let Some(cap) = self.ifndef_regex.captures(line) {
                let def = cap.get(1).unwrap();
                effective_defs.insert(def.as_str());
            }
        }

        let effective_defs = effective_defs.iter().map(ToString::to_string).collect();

        assert!((self.module_sets.len() as u32) < u32::MAX >> SPAN_SHIFT);

        let module_index = self.module_sets.len() + 1;
        let module_set = ComposableModuleDefinition {
            name: module_name.clone(),
            substituted_source,
            path: path.to_owned(),
            language,
            effective_defs,
            all_imports: imports.into_iter().map(|id| id.import).collect(),
            module_index,
            modules: Default::default(),
        };

        // todo invalidate dependent modules if this module already exists

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

    // shunt all data owned by a composable into a derived module
    fn add_composable_data<'a>(
        derived: &mut DerivedModule<'a>,
        composable: &'a ComposableModule,
        span_offset: usize,
    ) {
        derived.set_shader_source(&composable.module_ir, span_offset);

        for (h, ty) in composable.module_ir.types.iter() {
            if let Some(name) = &ty.name {
                if composable.owned_types.contains(name) {
                    derived.import_type(&h);
                }
            }
        }

        for (h, c) in composable.module_ir.constants.iter() {
            if let Some(name) = &c.name {
                if composable.owned_constants.contains(name) {
                    derived.import_const(&h);
                }
            }
        }

        for (h, v) in composable.module_ir.global_variables.iter() {
            if let Some(name) = &v.name {
                if composable.owned_vars.contains(name) {
                    derived.import_global(&h);
                }
            }
        }

        for (h_f, f) in composable.module_ir.functions.iter() {
            if let Some(name) = &f.name {
                if composable.owned_functions.contains(name) {
                    let span = composable.module_ir.functions.get_span(h_f);
                    derived.import_function(f, span);
                }
            }
        }

        derived.clear_shader_source();
    }

    // add an import (and recursive imports) into a derived module
    fn add_import<'a>(
        &'a self,
        derived: &mut DerivedModule<'a>,
        import: &String,
        shader_defs: &HashSet<String>,
        already_added: &mut HashSet<String>,
    ) {
        if already_added.contains(import) {
            return;
        }
        already_added.insert(import.clone());

        let import_module_set = self.module_sets.get(import).unwrap();
        let module = import_module_set.get(shader_defs).unwrap();

        for import in module.imports.iter() {
            self.add_import(derived, import, shader_defs, already_added);
        }

        Self::add_composable_data(
            derived,
            module,
            import_module_set.module_index << SPAN_SHIFT,
        );
    }

    // build required ComposableModules for a given set of shader_defs
    fn ensure_imports(
        &mut self,
        _source_name: &str,
        source_path: &str,
        source: &str,
        imports: Vec<ImportDefinition>,
        shader_defs: &HashSet<String>,
        already_built: &mut HashSet<String>,
    ) -> Result<(), ComposerError> {
        let mut to_build = Vec::default();

        for ImportDefinition { import, offset, .. } in imports.into_iter() {
            if already_built.contains(&import) {
                continue;
            }

            match self.module_sets.get(&import) {
                Some(module_set) => {
                    if module_set.get(shader_defs).is_none() {
                        let (_, _, imports) = self.preprocess_defs(
                            &import,
                            &module_set.substituted_source,
                            &module_set.path,
                            shader_defs,
                            true,
                        )?;
                        to_build.push((
                            import.clone(),
                            module_set.path.clone(),
                            module_set.substituted_source.clone(),
                            module_set.language,
                            imports,
                        ));
                    } else {
                        already_built.insert(import.clone());
                    }
                }
                None => {
                    return Err(ComposerError {
                        inner: ComposerErrorInner::ImportNotFound(import.clone(), offset),
                        source: ErrSource::Constructing {
                            path: source_path.to_owned(),
                            source: source.to_owned(),
                            offset: 0,
                        },
                    });
                }
            }
        }

        for (name, path, source, language, imports) in to_build.into_iter() {
            if !already_built.insert(name.clone()) {
                continue;
            };

            self.ensure_imports(&name, &path, &source, imports, shader_defs, already_built)?;
            let module = self.create_composable_module(
                &name,
                &path,
                &Self::decorate(&name),
                &source,
                language,
                shader_defs,
                true,
            )?;
            self.module_sets
                .get_mut(&name)
                .unwrap()
                .insert(shader_defs, module);
        }

        Ok(())
    }

    /// build a naga shader module
    pub fn make_naga_module(
        &mut self,
        source: &str,
        path: &str,
        shader_type: ShaderType,
        shader_defs: &[String],
    ) -> Result<naga::Module, ComposerError> {
        let shader_defs = shader_defs.iter().cloned().collect();

        let (name, modified_source, imports) =
            self.preprocess_defs("", &source, path, &shader_defs, false)?;

        let name = name.unwrap_or_default();
        let substituted_source = Self::sanitize_and_substitute_shader_string(source, &imports);

        self.ensure_imports(
            &name,
            path,
            &substituted_source,
            imports,
            &shader_defs,
            &mut Default::default(),
        )?;

        let composable = self.create_composable_module(
            &name,
            path,
            "",
            &substituted_source,
            shader_type.into(),
            &shader_defs,
            false,
        )?;

        let mut derived = DerivedModule::default();

        let mut already_added = Default::default();
        for import in composable.imports.iter() {
            self.add_import(&mut derived, import, &shader_defs, &mut already_added);
        }

        Self::add_composable_data(&mut derived, &composable, 0);

        let stage = match shader_type {
            ShaderType::GlslVertex => Some(naga::ShaderStage::Vertex),
            ShaderType::GlslFragment => Some(naga::ShaderStage::Fragment),
            _ => None,
        };

        let mut entry_points = Vec::default();
        derived.set_shader_source(&composable.module_ir, 0);
        for ep in composable.module_ir.entry_points.iter() {
            let mapped_func = derived.localize_function(&ep.function);
            entry_points.push(EntryPoint {
                name: ep.name.clone(),
                function: mapped_func,
                stage: stage.unwrap_or(ep.stage),
                early_depth_test: ep.early_depth_test,
                workgroup_size: ep.workgroup_size,
            });
        }

        let naga_module = naga::Module {
            entry_points,
            ..derived.into()
        };

        // println!("{}[{:?}] main module: {:#?}", name, shader_type, naga_module);

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
                                    path: path.to_owned(),
                                    source: substituted_source,
                                    offset: composable.start_offset,
                                },
                                _ => {
                                    let module_name = self
                                        .module_index
                                        .get(&module_index)
                                        .map(String::as_str)
                                        .unwrap();
                                    ErrSource::Composed(module_name.to_owned(), shader_defs)
                                }
                            }
                        }
                        None => ErrSource::Constructing {
                            path: path.to_owned(),
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
