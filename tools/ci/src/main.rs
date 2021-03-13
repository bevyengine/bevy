use xshell::cmd;

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
    cmd!("cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity")
    .run()
    .expect("Please fix clippy errors in output above.");
}
