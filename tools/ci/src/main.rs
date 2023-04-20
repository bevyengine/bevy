use xshell::{cmd, Shell};

/// The check that can be run in CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
struct Check {
    format: bool,
    clippy: bool,
    compile_fail: bool,
    test: bool,
    doc_test: bool,
    doc_check: bool,
    bench_check: bool,
    example_check: bool,
    compile_check: bool,
}

impl Check {
    /// Returns the complete check.
    fn all() -> Self {
        Self {
            format: true,
            clippy: true,
            compile_fail: true,
            test: true,
            doc_test: true,
            doc_check: true,
            bench_check: true,
            example_check: true,
            compile_check: true,
        }
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

    /// Returns the [`Check`] that corresponds to the given argument.
    fn from_argument(argument: &str) -> Option<Self> {
        match argument {
            "lints" => Some(Self {
                format: true,
                clippy: true,
                ..Default::default()
            }),
            "test" => Some(Self {
                test: true,
                ..Default::default()
            }),
            "doc" => Some(Self {
                doc_test: true,
                doc_check: true,
                ..Default::default()
            }),
            "compile" => Some(Self {
                compile_fail: true,
                bench_check: true,
                example_check: true,
                compile_check: true,
                ..Default::default()
            }),
            "format" => Some(Self {
                format: true,
                ..Default::default()
            }),
            "clippy" => Some(Self {
                clippy: true,
                ..Default::default()
            }),
            "compile-fail" => Some(Self {
                compile_fail: true,
                ..Default::default()
            }),
            "bench-check" => Some(Self {
                bench_check: true,
                ..Default::default()
            }),
            "example-check" => Some(Self {
                example_check: true,
                ..Default::default()
            }),
            "dock-check" => Some(Self {
                doc_check: true,
                ..Default::default()
            }),
            "doc-test" => Some(Self {
                doc_test: true,
                ..Default::default()
            }),
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

    if what_to_run.format {
        // See if any code needs to be formatted
        cmd!(sh, "cargo fmt --all -- --check")
            .run()
            .expect("Please run 'cargo fmt --all' to format your code.");
    }

    if what_to_run.clippy {
        // See if clippy has any complaints.
        // - Type complexity must be ignored because we use huge templates for queries
        cmd!(
            sh,
            "cargo clippy --workspace --all-targets --all-features -- {CLIPPY_FLAGS...}"
        )
        .run()
        .expect("Please fix clippy errors in output above.");
    }

    if what_to_run.compile_fail {
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

    if what_to_run.test {
        // Run tests (except doc tests and without building examples)
        cmd!(sh, "cargo test --workspace --lib --bins --tests --benches")
            .run()
            .expect("Please fix failing tests in output above.");
    }

    if what_to_run.doc_test {
        // Run doc tests
        cmd!(sh, "cargo test --workspace --doc")
            .run()
            .expect("Please fix failing doc-tests in output above.");
    }

    if what_to_run.doc_check {
        // Check that building docs work and does not emit warnings
        std::env::set_var("RUSTDOCFLAGS", "-D warnings");
        cmd!(
            sh,
            "cargo doc --workspace --all-features --no-deps --document-private-items"
        )
        .run()
        .expect("Please fix doc warnings in output above.");
    }

    if what_to_run.bench_check {
        let _subdir = sh.push_dir("benches");
        // Check that benches are building
        cmd!(sh, "cargo check --benches --target-dir ../target")
            .run()
            .expect("Failed to check the benches.");
    }

    if what_to_run.example_check {
        // Build examples and check they compile
        cmd!(sh, "cargo check --workspace --examples")
            .run()
            .expect("Please fix compiler errors for examples in output above.");
    }

    if what_to_run.compile_check {
        // Build bevy and check that it compiles
        cmd!(sh, "cargo check --workspace")
            .run()
            .expect("Please fix compiler errors in output above.");
    }
}
