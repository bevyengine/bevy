use std::{collections::HashMap, fs::File};

use serde::Serialize;
use tera::{Context, Tera};
use toml::Value;

fn main() {
    let examples = parse_examples();

    let examples_by_category: HashMap<String, Vec<Example>> =
        examples.into_iter().fold(HashMap::new(), |mut v, ex| {
            v.entry(ex.category.clone()).or_default().push(ex);
            v
        });
    let mut context = Context::new();
    context.insert("all_examples", &examples_by_category);
    Tera::new("examples/*.tpl.md")
        .expect("error parsing template")
        .render_to(
            "README.tpl.md",
            &context,
            File::create("examples/README_NEW.md").expect("error creating file"),
        )
        .expect("error rendering template");
}

#[derive(Debug, Serialize)]
struct Example {
    technical_name: String,
    path: String,
    name: String,
    description: String,
    category: String,
    wasm: bool,
}

fn parse_examples() -> Vec<Example> {
    let manifest_file = std::fs::read_to_string("Cargo.toml").unwrap();
    let manifest: HashMap<String, Value> = toml::from_str(&manifest_file).unwrap();
    let metadatas = manifest.get("metadata").as_ref().unwrap()["example"].clone();

    manifest["example"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|val| {
            let technical_name = val["name"].as_str().unwrap().to_string();
            metadatas.get(&technical_name).map(|metadata| Example {
                technical_name,
                path: val["path"].as_str().unwrap().to_string(),
                name: metadata["name"].as_str().unwrap().to_string(),
                description: metadata["description"].as_str().unwrap().to_string(),
                category: metadata["category"].as_str().unwrap().to_string(),
                wasm: metadata["wasm"].as_bool().unwrap(),
            })
        })
        .collect()
}
