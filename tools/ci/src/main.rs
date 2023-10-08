use xshell::{cmd, Shell};

use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Check: u32 {
        const FORMAT = 0b00000001;
        const CLIPPY = 0b00000010;
        const COMPILE_FAIL = 0b00000100;
        const TEST = 0b00001000;
        const DOC_TEST = 0b00010000;
        const DOC_CHECK = 0b00100000;
        const BENCH_CHECK = 0b01000000;
        const EXAMPLE_CHECK = 0b10000000;
        const COMPILE_CHECK = 0b100000000;
    }
}

const CLIPPY_FLAGS: [&str; 6] = [
    "-Wclippy::doc_markdown",
    "-Wclippy::redundant_else",
    "-Wclippy::match_same_arms",
    "-Wclippy::semicolon_if_nothing_returned",
    "-Wclippy::map_flatten",
    "-Dwarnings",
];

fn main() {
    // When run locally, results may differ from actual CI runs triggered by
    // .github/workflows/ci.yml
    // - Official CI runs latest stable
    // - Local runs use whatever the default Rust is locally

    let arguments = [
        ("lints", Check::FORMAT | Check::CLIPPY),
        ("test", Check::TEST),
        ("doc", Check::DOC_TEST | Check::DOC_CHECK),
        (
            "compile",
            Check::COMPILE_FAIL | Check::BENCH_CHECK | Check::EXAMPLE_CHECK | Check::COMPILE_CHECK,
        ),
        ("format", Check::FORMAT),
        ("clippy", Check::CLIPPY),
        ("compile-fail", Check::COMPILE_FAIL),
        ("bench-check", Check::BENCH_CHECK),
        ("example-check", Check::EXAMPLE_CHECK),
        ("doc-check", Check::DOC_CHECK),
        ("doc-test", Check::DOC_TEST),
    ];

    let what_to_run = if let Some(arg) = std::env::args().nth(1).as_deref() {
        if let Some((_, check)) = arguments.iter().find(|(str, _)| *str == arg) {
            *check
        } else {
            println!(
                "Invalid argument: {arg:?}.\nEnter one of: {}.",
                arguments[1..]
                    .iter()
                    .map(|(s, _)| s)
                    .fold(arguments[0].0.to_owned(), |c, v| c + ", " + v)
            );
            return;
        }
    } else {
        Check::all()
    };

    let sh = Shell::new().unwrap();

    if what_to_run.contains(Check::FORMAT) {
        // See if any code needs to be formatted
        cmd!(sh, "cargo fmt --all -- --check")
            .run()
            .expect("Please run 'cargo fmt --all' to format your code.");
    }

    if what_to_run.contains(Check::CLIPPY) {
        // See if clippy has any complaints.
        // - Type complexity must be ignored because we use huge templates for queries
        cmd!(
            sh,
            "cargo clippy --workspace --all-targets --all-features -- {CLIPPY_FLAGS...}"
        )
        .run()
        .expect("Please fix clippy errors in output above.");
    }

    if what_to_run.contains(Check::COMPILE_FAIL) {
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
        {
            // Macro Compile Fail Tests
            // Run tests (they do not get executed with the workspace tests)
            // - See crates/bevy_macros_compile_fail_tests/README.md
            let _subdir = sh.push_dir("crates/bevy_macros_compile_fail_tests");
            cmd!(sh, "cargo test --target-dir ../../target")
                .run()
                .expect("Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.");
        }
    }

    if what_to_run.contains(Check::TEST) {
        // Run tests (except doc tests and without building examples)
        cmd!(sh, "cargo test --workspace --lib --bins --tests --benches")
            .run()
            .expect("Please fix failing tests in output above.");
    }

    if what_to_run.contains(Check::DOC_TEST) {
        // Run doc tests
        cmd!(sh, "cargo test --workspace --doc")
            .run()
            .expect("Please fix failing doc-tests in output above.");
    }

    if what_to_run.contains(Check::DOC_CHECK) {
        // Check that building docs work and does not emit warnings
        std::env::set_var("RUSTDOCFLAGS", "-D warnings");
        cmd!(
            sh,
            "cargo doc --workspace --all-features --no-deps --document-private-items"
        )
        .run()
        .expect("Please fix doc warnings in output above.");
    }

    if what_to_run.contains(Check::BENCH_CHECK) {
        let _subdir = sh.push_dir("benches");
        // Check that benches are building
        cmd!(sh, "cargo check --benches --target-dir ../target")
            .run()
            .expect("Failed to check the benches.");
    }

    if what_to_run.contains(Check::EXAMPLE_CHECK) {
        // Build examples and check they compile
        cmd!(sh, "cargo check --workspace --examples")
            .run()
            .expect("Please fix compiler errors for examples in output above.");
    }

    if what_to_run.contains(Check::COMPILE_CHECK) {
        // Build bevy and check that it compiles
        cmd!(sh, "cargo check --workspace")
            .run()
            .expect("Please fix compiler errors in output above.");
    }
}
