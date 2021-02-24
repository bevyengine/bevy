use std::io::BufRead;

use duct::cmd;

fn main() {
    // When run locally, results may differ from actual CI runs triggered by .github/workflows/ci.yml
    // - Official CI runs latest stable
    // - Local runs use whatever the default Rust is locally

    // See if any code needs to be formatted
    println!("$ cargo fmt --all -- --check");
    let errput = cmd!("cargo", "fmt", "--all", "--", "--check")
        .stderr_capture()
        .run()
        .expect("Please run 'cargo fmt --all' to format your code.");
    // Capture stderr, filter out lines starting with "Warning:" that complain about nightly-only
    // options in rustfmt.toml -- we can remove this (and maybe switch back to `xshell` instead of
    // `duct`) once either rustfmt.toml doesn't cause warnings in stable or nightly rust, or rustfmt
    // learns an option to silence warnings provoked by the config file.
    errput
        .stderr
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.starts_with("Warning"))
        .for_each(|l| eprintln!("{}", l));

    // See if clippy has any complaints.
    // - Type complexity must be ignored because we use huge templates for queries
    // - `-A clippy::manual-strip` strip_prefix support was added in 1.45
    println!("$ cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::type_complexity -A clippy::manual-strip");
    cmd!(
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
        "-A",
        "clippy::type_complexity",
        "-A",
        "clippy::manual-strip"
    )
    .run()
    .expect("Please fix clippy errors in output above.");
}
