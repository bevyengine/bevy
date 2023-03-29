use std::{io::Write, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../src/");
    // TODO: this needs to run once
    // cargo +nightly rustdoc --target-dir target -- -Z unstable-options --output-format json
    // cargo +nightly rustdoc -p bevy_ecs -- -Z unstable-options -w json
    Command::new("cargo")
        .args([
            "+nightly",
            "rustdoc",
            "--manifest-path",
            "../",
            // "-p",
            // "bevy_ecs",
            "--",
            "-Zunstable-options",
            "-w",
            "json",
        ])
        .output()
        //TODO
        .expect("failed to generate json rustdocs -- did you install `rustup component add --toolchain nightly rust-docs-json`");
    // Bundle
    // SystemParam
    // WorldQuery
    // ScheduleLabel
    // SystemSet
    // Optional: Resource
    // Component
    // States

    // jq -r '.. | objects | select(.name == "SystemParam" and has("docs")) | .docs' target/doc/bevy_ecs.json > system_param.md
    let traits_to_document = [
        "Bundle",
        "SystemParam",
        "WorldQuery",
        "ScheduleLabel",
        "SystemSet",
        // "Resource",
        "Component",
        "States",
    ];
    let traits_doc_filenames = [
        "bundle.md",
        "system_param.md",
        "world_query.md",
        "schedule_label.md",
        "system_set.md",
        // "resource.md",
        "component.md",
        "states.md",
    ];
    use std::env;
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let trait_docs = traits_to_document
        .into_iter()
        .flat_map(|x| {
            Command::new("jq")
                .arg("-r")
                .arg(format!(
                    r#".. | objects | select(.name == "{x}" and has("docs")) | .docs"#
                ))
                .arg(format!("{out_dir}/../../../../doc/bevy_ecs.json"))
                //TODO: do something so it doesn't output it
                .output()
            //TODO: Add # to all headings
            //TODO: Remove links at the bottom, as they won't work.
        })
        .map(|x| x.stdout);

    let traits_doc_paths = traits_doc_filenames
        .into_iter()
        .map(|x| PathBuf::from(format!("{manifest_dir}/../doc/{x}")))
        .collect::<Vec<_>>();

    std::fs::create_dir_all(traits_doc_paths[0].parent().unwrap()).unwrap();
    for (trait_doc_path, trait_doc) in traits_doc_paths.into_iter().zip(trait_docs.into_iter()) {
        if !std::path::Path::exists(&trait_doc_path) {
            std::fs::File::create(&trait_doc_path).unwrap();
        }
        std::fs::File::options()
            .append(false)
            .truncate(true)
            .write(true)
            .open(trait_doc_path)
            //TODO
            .expect("ERROR")
            .write_all(trait_doc.as_slice())
            .unwrap();
    }
}
