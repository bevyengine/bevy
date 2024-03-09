use std::{cmp::Ordering, fs::File};

use serde::Serialize;
use tera::{Context, Tera};
use toml_edit::{Document, Key, Table, Value};

use crate::Command;

#[derive(Debug, Serialize, PartialEq, Eq)]
struct Feature {
    name: String,
    description: String,
    is_default: bool,
}

impl Ord for Feature {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Feature {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_features(what_to_run: Command) -> Vec<Feature> {
    let features = get_manifest_features();
    let default = get_default_features(&features);

    features
        .get_values()
        .iter()
        .flat_map(|(key, _)| {
            let key = key[0];

            if key == "default" {
                None
            } else {
                let key_name = get_key_name(key);
                let key_prefix = key.leaf_decor().prefix().unwrap().as_str();
                if what_to_run.contains(Command::CHECK_MISSING) {
                    match key_prefix {
                        Some(description) => create_feature(&default, key_name, description),
                        None => panic!("Missing description for feature {key_name}"),
                    }
                } else {
                    key_prefix
                        .map(|description| create_feature(&default, key_name, description))
                        .unwrap()
                }
            }
        })
        .collect()
}

fn get_manifest_features() -> Table {
    let manifest_file = std::fs::read_to_string("Cargo.toml").unwrap();
    let manifest = manifest_file.parse::<Document>().unwrap();
    let features = manifest["features"].as_table().unwrap().clone();
    features
}

fn create_feature(default: &[&str], name: &str, description: &str) -> Option<Feature> {
    let description = description.to_string();
    if let Some(stripped_description) = description
        .strip_prefix("\n# ")
        .and_then(|d| d.strip_suffix('\n'))
    {
        let is_default = default.contains(&name);
        let description = get_description(stripped_description, name);
        Some(Feature {
            is_default,
            name: name.to_string(),
            description: description.to_string(),
        })
    } else {
        panic!("Missing description for feature {name}");
    }
}

fn get_description<'a>(description: &'a str, name: &str) -> &'a str {
    if !description.starts_with("\n# ") || !description.ends_with('\n') {
        panic!("Missing description for feature {name}");
    }
    description
        .strip_prefix("\n# ")
        .unwrap()
        .strip_suffix('\n')
        .unwrap()
}

fn get_key_name(key: &Key) -> &str {
    key.as_repr().unwrap().as_raw().as_str().unwrap()
}

fn get_default_features(features: &Table) -> Vec<&str> {
    let features_to_array = |name| features.get(name).unwrap().as_array().unwrap();
    features_to_array("default")
        .iter()
        .flat_map(|v: &Value| {
            let feature_name = v.as_str().unwrap();
            let features_to_array = |name| features.get(name).unwrap().as_array().unwrap();
            let map = features_to_array(feature_name)
                .iter()
                .map(|v: &Value| v.as_str().unwrap());
            std::iter::once(feature_name).chain(map)
        })
        .collect()
}

pub(crate) fn check(command: Command) {
    let mut features = parse_features(command);
    features.sort();

    if command.contains(Command::UPDATE) {
        let mut context = Context::new();
        context.insert("features", &features);
        let file = File::create("docs/cargo_features.md").expect("error creating file");
        Tera::new("docs-template/*.md.tpl")
            .expect("error parsing template")
            .render_to("features.md.tpl", &context, file)
            .expect("error rendering template");
    }
}
