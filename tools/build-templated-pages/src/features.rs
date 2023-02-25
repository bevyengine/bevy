use std::{cmp::Ordering, fs::File};

use serde::Serialize;
use tera::{Context, Tera};
use toml_edit::Document;

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

fn parse_features(panic_on_missing: bool) -> Vec<Feature> {
    let manifest_file = std::fs::read_to_string("Cargo.toml").unwrap();
    let manifest = manifest_file.parse::<Document>().unwrap();

    let features = manifest["features"].as_table().unwrap();
    let default: Vec<_> = features
        .get("default")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|v| {
            std::iter::once(v.as_str().unwrap().to_string()).chain(
                features
                    .get(v.as_str().unwrap())
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string()),
            )
        })
        .collect();

    features
        .get_values()
        .iter()
        .flat_map(|(key, _)| {
            let key = key[0];

            if key == "default" {
                None
            } else {
                let name = key
                    .as_repr()
                    .unwrap()
                    .as_raw()
                    .as_str()
                    .unwrap()
                    .to_string();
                if let Some(description) = key.decor().prefix() {
                    let description = description.as_str().unwrap().to_string();
                    if !description.starts_with("\n# ") || !description.ends_with('\n') {
                        panic!("Missing description for feature {name}");
                    }
                    let description = description
                        .strip_prefix("\n# ")
                        .unwrap()
                        .strip_suffix('\n')
                        .unwrap()
                        .to_string();
                    Some(Feature {
                        is_default: default.contains(&name),
                        name,
                        description,
                    })
                } else if panic_on_missing {
                    panic!("Missing description for feature {name}");
                } else {
                    None
                }
            }
        })
        .collect()
}

pub(crate) fn check(what_to_run: Command) {
    let mut features = parse_features(what_to_run.contains(Command::CHECK_MISSING));
    features.sort();

    if what_to_run.contains(Command::UPDATE) {
        let mut context = Context::new();
        context.insert("features", &features);
        Tera::new("docs-template/*.md.tpl")
            .expect("error parsing template")
            .render_to(
                "features.md.tpl",
                &context,
                File::create("docs/cargo_features.md").expect("error creating file"),
            )
            .expect("error rendering template");
    }
}
