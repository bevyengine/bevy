use core::cmp::Ordering;
use std::fs::File;

use serde::Serialize;
use tera::{Context, Tera};
use toml_edit::DocumentMut;

use crate::Command;

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
struct Feature {
    name: String,
    description: String,
    is_profile: bool,
    is_collection: bool,
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
    let manifest = manifest_file.parse::<DocumentMut>().unwrap();

    let features = manifest["features"].as_table().unwrap();

    features
        .get_values()
        .iter()
        .flat_map(|(key, value)| {
            let key = key[0];

            if key == "default" {
                let values = value
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let description = format!("The full default Bevy experience. This is a combination of the following profiles: {values}");

                Some(Feature {
                    is_profile: true,
                    is_collection: false,
                    name: "default".to_string(),
                    description,
                })
            } else {
                let name = key
                    .as_repr()
                    .unwrap()
                    .as_raw()
                    .as_str()
                    .unwrap()
                    .to_string();
                if let Some(description) = key.leaf_decor().prefix() {
                    let description = description.as_str().unwrap().to_string();
                    if !description.starts_with("\n# ") || !description.ends_with('\n') {
                        panic!("Missing description for feature {name}");
                    }
                    let mut description = description
                        .strip_prefix("\n# ")
                        .unwrap()
                        .strip_suffix('\n')
                        .unwrap()
                        .to_string();
                    let is_profile = if let Some(trimmed) = description.strip_prefix("PROFILE: ") {
                        description = trimmed.to_string();
                        true
                    } else {
                        false
                    };
                    let is_collection =
                        if let Some(trimmed) = description.strip_prefix("COLLECTION: ") {
                            description = trimmed.to_string();
                            true
                        } else {
                            false
                        };

                    if is_collection {
                        let features = value
                            .as_array()
                            .unwrap()
                            .iter()
                            .flat_map(|v| v.as_str().map(|s| format!("`{}`", s)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        description.push_str(&format!(" **Feature set:** {}.", &features));
                    }

                    Some(Feature {
                        is_profile,
                        is_collection,
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
    let features = parse_features(what_to_run.contains(Command::CHECK_MISSING));
    let mut sorted_features = features.clone();
    sorted_features.sort();

    if what_to_run.contains(Command::UPDATE) {
        panic!("this panic can be removed, if CARGO_PKG_VERSION is also the bevy's workspace version"); // TODO: note this panic!
        let long_version = std::env::var("CARGO_PKG_VERSION").unwrap(); 
        let version: _ = long_version.as_str().rsplit_once('.').unwrap().0;

        let mut context = Context::new();
        context.insert("features", &features);
        context.insert("sorted_features", &sorted_features);
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
