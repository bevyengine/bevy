use std::{borrow::Cow, collections::HashMap, ops::Range};

use codespan_reporting::{
    diagnostic::{Diagnostic, Label},
    files::SimpleFile,
    term,
};
use thiserror::Error;
use tracing::trace;

use super::{
    preprocess::{PreprocessOutput, PreprocessorMetaData},
    Composer, ShaderDefValue,
};
use crate::{compose::SPAN_SHIFT, redirect::RedirectError};

#[derive(Debug)]
pub enum ErrSource {
    Module {
        name: String,
        offset: usize,
        defs: HashMap<String, ShaderDefValue>,
    },
    Constructing {
        path: String,
        source: String,
        offset: usize,
    },
}

impl ErrSource {
    pub fn path<'a>(&'a self, composer: &'a Composer) -> &'a String {
        match self {
            ErrSource::Module { name, .. } => &composer.module_sets.get(name).unwrap().file_path,
            ErrSource::Constructing { path, .. } => path,
        }
    }

    pub fn source<'a>(&'a self, composer: &'a Composer) -> Cow<'a, String> {
        match self {
            ErrSource::Module { name, defs, .. } => {
                let raw_source = &composer.module_sets.get(name).unwrap().sanitized_source;
                let Ok(PreprocessOutput {
                    preprocessed_source: source,
                    meta: PreprocessorMetaData { imports, .. },
                }) = composer
                    .preprocessor
                    .preprocess(raw_source, defs, composer.validate)
                    else {
                        return Default::default()
                    };

                let Ok(source) = composer
                    .substitute_shader_string(&source, &imports)
                    else { return Default::default() };

                Cow::Owned(source)
            }
            ErrSource::Constructing { source, .. } => Cow::Borrowed(source),
        }
    }

    pub fn offset(&self) -> usize {
        match self {
            ErrSource::Module { offset, .. } | ErrSource::Constructing { offset, .. } => *offset,
        }
    }
}

#[derive(Debug, Error)]
#[error("Composer error: {inner}")]
pub struct ComposerError {
    #[source]
    pub inner: ComposerErrorInner,
    pub source: ErrSource,
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
    #[error("'#else' without preceding condition.")]
    ElseWithoutCondition(usize),
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
    #[error("Invalid value for `#define`d shader def {name}: {value}")]
    InvalidShaderDefDefinitionValue {
        name: String,
        value: String,
        pos: usize,
    },
    #[error("#define statements are only allowed at the start of the top-level shaders")]
    DefineInModule(usize),
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

// impl<'a> FusedIterator for ErrorSources<'a> {}

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

        let files = SimpleFile::new(path, source.as_str());
        let config = term::Config::default();
        #[cfg(any(test, target_arch = "wasm32"))]
        let mut writer = term::termcolor::NoColor::new(Vec::new());
        #[cfg(not(any(test, target_arch = "wasm32")))]
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
            | ComposerErrorInner::ElseWithoutCondition(pos)
            | ComposerErrorInner::UnknownShaderDef { pos, .. }
            | ComposerErrorInner::UnknownShaderDefOperator { pos, .. }
            | ComposerErrorInner::InvalidShaderDefComparisonValue { pos, .. }
            | ComposerErrorInner::OverrideNotVirtual { pos, .. }
            | ComposerErrorInner::GlslInvalidVersion(pos)
            | ComposerErrorInner::DefineInModule(pos)
            | ComposerErrorInner::InvalidShaderDefDefinitionValue { pos, .. } => {
                (vec![Label::primary((), *pos..*pos)], vec![])
            }
            ComposerErrorInner::WgslBackError(e) => {
                return format!("{path}: wgsl back error: {e}");
            }
            ComposerErrorInner::GlslBackError(e) => {
                return format!("{path}: glsl back error: {e}");
            }
            ComposerErrorInner::InconsistentShaderDefValue { def } => {
                return format!("{path}: multiple inconsistent shader def values: '{def}'");
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
