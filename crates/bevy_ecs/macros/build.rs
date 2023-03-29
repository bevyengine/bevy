use std::{io::Write, path::PathBuf, process::Command};

fn main() {
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
    //
    // jq -r '.. | objects | select(.name == "SystemParam" and has("docs")) | .docs' target/doc/bevy_ecs.json > system_param.md

    use std::env;
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut jq_command = Command::new("jq");
    let system_param_doc = jq_command
        .arg("-r")
        .arg(r#".. | objects | select(.name == "SystemParam" and has("docs")) | .docs"#)
        .arg(format!("{out_dir}/../../../../doc/bevy_ecs.json"));
    assert!(system_param_doc.status().unwrap().success());

    let system_param_doc = system_param_doc
        //TODO: do something so it doesn't output it
        .output()
        .expect("failed to extract SystemParam");
    // assert!(system_param_doc.status.success());
    //TODO: Add # to all headings
    //TODO: Remove links at the bottom, as they won't work.

    let system_param_doc = system_param_doc.stdout;
    let system_param_doc_path = format!("{manifest_dir}/../doc/{}", "system_param.md");
    let system_param_doc_path = PathBuf::from(system_param_doc_path);

    std::fs::create_dir_all(system_param_doc_path.parent().unwrap()).unwrap();
    if !std::path::Path::exists(&system_param_doc_path) {
        std::fs::File::create(&system_param_doc_path).unwrap();
    }
    std::fs::File::options()
        .append(false)
        .truncate(true)
        .write(true)
        .open(system_param_doc_path)
        //TODO
        .expect("ERROR")
        .write_all(system_param_doc.as_slice())
        .unwrap();
}
