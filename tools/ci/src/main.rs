use std::path::Path;

use regex::Regex;
use xshell::{cmd, pushd, read_dir};

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml
    // - Official CI runs latest stable
    // - Local runs use whatever the default Rust is locally

    // Iterate through error codes and add them to the table of contents in the README.md
    // This Regex matches a 'B' followed by four digits with a '.md' file extension
    let error_doc_title_pattern =
        Regex::new(r"B\d\d\d\d\.md").expect("Failed to create Regex, check the syntax.");
    let path = Path::new("../../errors"); // Traverse up and into the errors directory
    let errors_docs_dir = read_dir(path).expect("Failed to read errors docs dir.");
    errors_docs_dir.iter().for_each(|doc_path| {
        let file_name = doc_path
            .file_name()
            .expect("Failed to parse file name from doc_path");
        for regex_capture in error_doc_title_pattern.captures_iter(file_name.to_str().unwrap()) {
            // We should only have one capture
            println!("{}", &regex_capture[0]);
        }
    });

    // See if any code needs to be formatted
    cmd!("cargo fmt --all -- --check")
        .run()
        .expect("Please run 'cargo fmt --all' to format your code.");

    // See if clippy has any complaints.
    // - Type complexity must be ignored because we use huge templates for queries
    cmd!("cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity")
        .run()
        .expect("Please fix clippy errors in output above.");

    // Run UI tests (they do not get executed with the workspace tests)
    // - See crates/bevy_ecs_compile_fail_tests/README.md
    {
        let _bevy_ecs_compile_fail_tests = pushd("crates/bevy_ecs_compile_fail_tests")
            .expect("Failed to navigate to the 'bevy_ecs_compile_fail_tests' crate");
        cmd!("cargo test")
            .run()
            .expect("Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.");
    }
}
