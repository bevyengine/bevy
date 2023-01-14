use pulldown_cmark::{Event, Options, Parser, Tag};
use std::collections::{HashMap, HashSet};
use toml::Value;

#[derive(Debug)]
enum FeaturesDocError {
    NoDefaultFeaturesTable,
    NoOptionalFeaturesTable,
    ManifestParsingFailed(String),
    DocParsingFailed(String),
    UndocumentedFeature(String),
    NonExistantFeature(String),
    FeatureNotDefault(String),
    FeatureIsDefault(String),
}

#[derive(Default)]
struct DocTables {
    sections: HashMap<String, Vec<Vec<String>>>,
}

impl DocTables {
    fn first_col_contains(&self, section: &str, val: &str) -> bool {
        self.iter_first_col(section)
            .any(|first_cell| first_cell == val)
    }

    fn iter_first_col(&self, section: &str) -> impl Iterator<Item = &String> + '_ {
        self.sections
            .get(section)
            .unwrap()
            .iter()
            .flat_map(|row| row.get(0))
    }

    fn has_section(&self, section: &str) -> bool {
        self.sections.contains_key(section)
    }
}

struct ManifestFeatures {
    default: HashSet<String>,
    all: HashSet<String>,
}

enum ParserState {
    None,
    Heading,
    Section,
    TableRow,
    TableCell,
}

const DEFAULT_FEATURES_HEADING: &str = "Default Features";
const OPTIONAL_FEATURES_HEADING: &str = "Optional Features";

fn main() -> Result<(), FeaturesDocError> {
    let manifest_features = parse_manifest()?;
    let doc_tables = parse_doc()?;

    if !doc_tables.has_section(DEFAULT_FEATURES_HEADING) {
        return Err(FeaturesDocError::NoDefaultFeaturesTable);
    }
    if !doc_tables.has_section(OPTIONAL_FEATURES_HEADING) {
        return Err(FeaturesDocError::NoOptionalFeaturesTable);
    }

    let mut errors = vec![];

    // Check for features that are defined in the manifest, but nowhere to be found in the docs.
    for feature in &manifest_features.all {
        let in_default = doc_tables.first_col_contains(DEFAULT_FEATURES_HEADING, feature);
        let in_optional = doc_tables.first_col_contains(OPTIONAL_FEATURES_HEADING, feature);

        if !in_default && !in_optional {
            errors.push(FeaturesDocError::UndocumentedFeature(feature.clone()));
        }
    }

    // Check for features in the docs that are not defined in the manifest or are miscategorized.

    for feature in doc_tables.iter_first_col(DEFAULT_FEATURES_HEADING) {
        let default = manifest_features.default.contains(feature);
        let is_feature = manifest_features.all.contains(feature);

        match (default, is_feature) {
            (false, false) => errors.push(FeaturesDocError::NonExistantFeature(feature.clone())),
            (false, true) => errors.push(FeaturesDocError::FeatureNotDefault(feature.clone())),
            _ => {}
        }
    }

    for feature in doc_tables.iter_first_col(OPTIONAL_FEATURES_HEADING) {
        let default = manifest_features.default.contains(feature);
        let is_feature = manifest_features.all.contains(feature);

        match (default, is_feature) {
            (true, _) => errors.push(FeaturesDocError::FeatureIsDefault(feature.clone())),
            (_, false) => errors.push(FeaturesDocError::NonExistantFeature(feature.clone())),
            _ => {}
        }
    }

    if !errors.is_empty() {
        for error in errors {
            eprintln!("{:?}", error);
        }
        std::process::exit(1);
    }

    Ok(())
}

fn parse_doc() -> Result<DocTables, FeaturesDocError> {
    let doc_file = std::fs::read_to_string("docs/cargo_features.md")
        .map_err(|e| FeaturesDocError::DocParsingFailed(e.to_string()))?;

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let markdown = Parser::new_ext(&doc_file, options);

    let mut state = ParserState::None;
    let mut current_section: Option<String> = None;
    let mut current_row = None;
    let mut tables = DocTables::default();

    for event in markdown {
        match &event {
            Event::Start(tag) => match tag {
                Tag::Heading(_heading_level, _fragment_identifier, _class_list) => {
                    state = ParserState::Heading;
                }
                Tag::Table(_column_text_alignment_list) => {
                    if let Some(ref section) = &current_section {
                        tables.sections.insert(section.to_string(), vec![]);
                    }
                }
                Tag::TableRow => {
                    state = ParserState::TableRow;
                    current_row = Some(vec![]);
                }
                Tag::TableCell => {
                    state = ParserState::TableCell;
                }
                _ => {}
            },
            Event::Text(text) => match (&state, &current_section) {
                (ParserState::Heading, _) => {
                    state = ParserState::Section;
                    current_section = Some(text.to_string());
                }
                (ParserState::TableCell, Some(ref _section)) => {
                    state = ParserState::TableCell;
                    if let Some(ref mut row) = current_row {
                        row.push(text.to_string());
                    }
                }
                _ => {}
            },
            Event::End(Tag::TableRow) => {
                if let (Some(ref section), Some(row)) = (&current_section, &current_row) {
                    tables
                        .sections
                        .get_mut(section)
                        .ok_or_else(|| {
                            FeaturesDocError::DocParsingFailed(
                                "table row ended, but corresponding section not found".to_string(),
                            )
                        })?
                        .push(row.to_vec());
                }
            }
            _ => (),
        };
    }

    Ok(tables)
}

fn parse_manifest() -> Result<ManifestFeatures, FeaturesDocError> {
    let manifest_file = std::fs::read_to_string("Cargo.toml")
        .map_err(|e| FeaturesDocError::ManifestParsingFailed(e.to_string()))?;

    let manifest: HashMap<String, Value> = toml::from_str(&manifest_file)
        .map_err(|e| FeaturesDocError::ManifestParsingFailed(e.to_string()))?;

    let features = manifest
        .get("features")
        .ok_or_else(|| FeaturesDocError::ManifestParsingFailed("No features section".to_string()))?
        .as_table()
        .ok_or_else(|| {
            FeaturesDocError::ManifestParsingFailed("features section invalid".to_string())
        })?;

    let mut default = HashSet::new();
    let mut all = HashSet::new();

    for (feature, enables) in features.iter() {
        if feature == "default" {
            default = enables
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect();
        } else {
            all.insert(feature.to_string());
        }
    }

    Ok(ManifestFeatures { default, all })
}
