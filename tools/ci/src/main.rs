use xshell::{cmd, Shell};

/// The checks that can be run in CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Check {
    Format,
    Clippy,
    CompileFail,
    Test,
    DocTest,
    DocCheck,
    BenchCheck,
    ExampleCheck,
    CompileCheck,
}

impl Check {
    /// Returns the complete set of checks.
    fn all() -> Vec<Check> {
        vec![
            Check::Format,
            Check::Clippy,
            Check::CompileFail,
            Check::Test,
            Check::DocTest,
            Check::DocCheck,
            Check::BenchCheck,
            Check::ExampleCheck,
            Check::CompileCheck,
        ]
    }

    /// Returns all supported arguments.
    fn all_arguments() -> Vec<&'static str> {
        vec![
            "lints",
            "test",
            "doc",
            "compile",
            "format",
            "clippy",
            "compile-fail",
            "bench-check",
            "example-check",
            "dock-check",
            "doc-test",
        ]
    }

    /// Returns the collection of [`Check`] that corresponds to the given argument.
    fn from_argument(argument: &str) -> Option<Vec<Check>> {
        match argument {
            "lints" => Some(vec![Check::Format, Check::Clippy]),
            "test" => Some(vec![Check::Test]),
            "doc" => Some(vec![Check::DocTest, Check::DocCheck]),
            "compile" => Some(vec![
                Check::CompileFail,
                Check::BenchCheck,
                Check::ExampleCheck,
                Check::CompileCheck,
            ]),
            "format" => Some(vec![Check::Format]),
            "clippy" => Some(vec![Check::Clippy]),
            "compile-fail" => Some(vec![Check::CompileFail]),
            "bench-check" => Some(vec![Check::BenchCheck]),
            "example-check" => Some(vec![Check::ExampleCheck]),
            "dock-check" => Some(vec![Check::DocCheck]),
            "doc-test" => Some(vec![Check::DocTest]),
            _ => None,
        }
    }
}

const CLIPPY_FLAGS: [&str; 8] = [
    "-Aclippy::type_complexity",
    "-Wclippy::doc_markdown",
    "-Wclippy::redundant_else",
    "-Wclippy::match_same_arms",
    "-Wclippy::semicolon_if_nothing_returned",
    "-Wclippy::explicit_iter_loop",
    "-Wclippy::map_flatten",
    "-Dwarnings",
];

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml
    // - Official CI runs latest stable
    // - Local runs use whatever the default Rust is locally

    let what_to_run = if let Some(arg) = std::env::args().nth(1).as_deref() {
        if let Some(checks) = Check::from_argument(arg) {
            checks
        } else {
            println!(
                "Invalid argument: {arg:?}.\nEnter one of: {}.",
                Check::all_arguments().join(", "),
            );
            return;
        }
    } else {
        Check::all()
    };

    let sh = Shell::new().unwrap();

    if what_to_run.contains(&Check::Format) {
        // See if any code needs to be formatted
        cmd!(sh, "cargo fmt --all -- --check")
            .run()
            .expect("Please run 'cargo fmt --all' to format your code.");
    }

    if what_to_run.contains(&Check::Clippy) {
        // See if clippy has any complaints.
        // - Type complexity must be ignored because we use huge templates for queries
        cmd!(
            sh,
            "cargo clippy --workspace --all-targets --all-features -- {CLIPPY_FLAGS...}"
        )
        .run()
        .expect("Please fix clippy errors in output above.");
    }

    if what_to_run.contains(&Check::CompileFail) {
        {
            // ECS Compile Fail Tests
            // Run UI tests (they do not get executed with the workspace tests)
            // - See crates/bevy_ecs_compile_fail_tests/README.md
            let _subdir = sh.push_dir("crates/bevy_ecs_compile_fail_tests");
            cmd!(sh, "cargo test --target-dir ../../target")
                .run()
                .expect("Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.");
        }
        {
            // Reflect Compile Fail Tests
            // Run tests (they do not get executed with the workspace tests)
            // - See crates/bevy_reflect_compile_fail_tests/README.md
            let _subdir = sh.push_dir("crates/bevy_reflect_compile_fail_tests");
            cmd!(sh, "cargo test --target-dir ../../target")
                .run()
                .expect("Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.");
        }
    }

    if what_to_run.contains(&Check::Test) {
        // Run tests (except doc tests and without building examples)
        cmd!(sh, "cargo test --workspace --lib --bins --tests --benches")
            .run()
            .expect("Please fix failing tests in output above.");
    }

    if what_to_run.contains(&Check::DocTest) {
        // Run doc tests
        cmd!(sh, "cargo test --workspace --doc")
            .run()
            .expect("Please fix failing doc-tests in output above.");
    }

    if what_to_run.contains(&Check::DocCheck) {
        // Check that building docs work and does not emit warnings
        std::env::set_var("RUSTDOCFLAGS", "-D warnings");
        cmd!(
            sh,
            "cargo doc --workspace --all-features --no-deps --document-private-items"
        )
        .run()
        .expect("Please fix doc warnings in output above.");
    }

    if what_to_run.contains(&Check::BenchCheck) {
        let _subdir = sh.push_dir("benches");
        // Check that benches are building
        cmd!(sh, "cargo check --benches --target-dir ../target")
            .run()
            .expect("Failed to check the benches.");
    }

    if what_to_run.contains(&Check::ExampleCheck) {
        // Build examples and check they compile
        cmd!(sh, "cargo check --workspace --examples")
            .run()
            .expect("Please fix compiler errors for examples in output above.");
    }

    if what_to_run.contains(&Check::CompileCheck) {
        // Build bevy and check that it compiles
        cmd!(sh, "cargo check --workspace")
            .run()
            .expect("Please fix compiler errors in output above.");
    }
}
