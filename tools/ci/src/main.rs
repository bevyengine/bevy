use xshell::{cmd, pushd};

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml
    // - Official CI runs latest stable
    // - Local runs use whatever the default Rust is locally

    // See if any code needs to be formatted
    cmd!("cargo fmt --all -- --check")
        .run()
        .expect("Please run 'cargo fmt --all' to format your code.");

    // See if clippy has any complaints.
    // - Type complexity must be ignored because we use huge templates for queries
    cmd!("cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity -W clippy::doc_markdown")
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

    // These tests are already run on the CI
    // Using a double-negative here allows end-users to have a nicer experience
    // as we can pass in the extra argument to the CI script
    let args: Vec<String> = std::env::args().collect();
    if args.get(1) != Some(&"nonlocal".to_string()) {
        // Run tests
        cmd!("cargo test --workspace")
            .run()
            .expect("Please fix failing tests in output above.");

        // Run doc tests: these are ignored by `cargo test`
        cmd!("cargo test --doc --workspace")
            .run()
            .expect("Please fix failing doc-tests in output above.");
    }
}
