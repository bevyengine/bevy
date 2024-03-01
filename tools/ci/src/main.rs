//! CI script used for Bevy.

use bitflags::bitflags;
use core::panic;
use std::collections::BTreeMap;
use xshell::{cmd, Cmd, Shell};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
        const CFG_CHECK = 0b1000000000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Flag: u32 {
        const KEEP_GOING = 0b00000001;
    }
}

// None of the CI tests require any information at runtime other than the options that have been set,
// which is why all of these are 'static; we could easily update this to use more flexible types.
struct CITest<'a> {
    command: Cmd<'a>,                            // The command to execute
    failure_message: &'static str,               // The message to display if it fails
    subdir: Option<&'static str>,                // The subdirectory path to run the command within
    env_vars: Vec<(&'static str, &'static str)>, // Environment variables that need to be set before the command runs
}

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
        ("cfg-check", Check::CFG_CHECK),
        ("doc-check", Check::DOC_CHECK),
        ("doc-test", Check::DOC_TEST),
    ];

    let flag_arguments = [("--keep-going", Flag::KEEP_GOING)];

    let (mut checks, mut flags) = (Check::empty(), Flag::empty());
    for arg in std::env::args().skip(1) {
        if let Some((_, flag)) = flag_arguments.iter().find(|(flag_arg, _)| *flag_arg == arg) {
            flags.insert(*flag);
            continue;
        }
        if let Some((_, check)) = arguments.iter().find(|(check_arg, _)| *check_arg == arg) {
            checks.insert(*check);
            continue;
        }
        println!(
            "Invalid argument: {arg:?}.\nEnter one of: {}.",
            arguments[1..]
                .iter()
                .map(|(s, _)| s)
                .fold(arguments[0].0.to_owned(), |c, v| c + ", " + v)
        );
        return;
    }

    // If no checks are specified, run every check
    if checks.is_empty() {
        checks = Check::all();
    }

    let sh = Shell::new().unwrap();

    // Each check contains a 'battery' (vector) that can include more than one command, but almost all of them
    // just contain a single command.
    let mut test_suite: BTreeMap<Check, Vec<CITest>> = BTreeMap::new();

    if checks.contains(Check::FORMAT) {
        // See if any code needs to be formatted
        test_suite.insert(
            Check::FORMAT,
            vec![CITest {
                command: cmd!(sh, "cargo fmt --all -- --check"),
                failure_message: "Please run 'cargo fmt --all' to format your code.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::CLIPPY) {
        // See if clippy has any complaints.
        // - Type complexity must be ignored because we use huge templates for queries
        test_suite.insert(
            Check::CLIPPY,
            vec![CITest {
                command: cmd!(
                    sh,
                    "cargo clippy --workspace --all-targets --all-features -- -Dwarnings"
                ),
                failure_message: "Please fix clippy errors in output above.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::COMPILE_FAIL) {
        let mut args = vec!["--target-dir", "../../target"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--no-fail-fast");
        }

        // ECS Compile Fail Tests
        // Run UI tests (they do not get executed with the workspace tests)
        // - See crates/bevy_ecs_compile_fail_tests/README.md

        // (These must be cloned because of move semantics in `cmd!`)
        let args_clone = args.clone();

        test_suite.insert(Check::COMPILE_FAIL, vec![CITest {
            command: cmd!(sh, "cargo test {args_clone...}"),
            failure_message: "Compiler errors of the ECS compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            subdir: Some("crates/bevy_ecs_compile_fail_tests"),
            env_vars: vec![],
        }]);

        // Reflect Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_reflect_compile_fail_tests/README.md
        let args_clone = args.clone();

        test_suite.entry(Check::COMPILE_FAIL).and_modify(|tests| tests.push( CITest {
            command: cmd!(sh, "cargo test {args_clone...}"),
            failure_message: "Compiler errors of the Reflect compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            subdir: Some("crates/bevy_reflect_compile_fail_tests"),
            env_vars: vec![],
        }));

        // Macro Compile Fail Tests
        // Run tests (they do not get executed with the workspace tests)
        // - See crates/bevy_macros_compile_fail_tests/README.md
        let args_clone = args.clone();

        test_suite.entry(Check::COMPILE_FAIL).and_modify(|tests| tests.push( CITest {
            command: cmd!(sh, "cargo test {args_clone...}"),
            failure_message: "Compiler errors of the macros compile fail tests seem to be different than expected! Check locally and compare rust versions.",
            subdir: Some("crates/bevy_macros_compile_fail_tests"),
            env_vars: vec![],
        }));
    }

    if checks.contains(Check::TEST) {
        // Run tests (except doc tests and without building examples)
        let mut args = vec!["--workspace", "--lib", "--bins", "--tests", "--benches"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--no-fail-fast");
        }

        test_suite.insert(
            Check::TEST,
            vec![CITest {
                command: cmd!(sh, "cargo test {args...}"),
                failure_message: "Please fix failing tests in output above.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::DOC_TEST) {
        // Run doc tests
        let mut args = vec!["--workspace", "--doc"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--no-fail-fast");
        }

        test_suite.insert(
            Check::DOC_TEST,
            vec![CITest {
                command: cmd!(sh, "cargo test {args...}"),
                failure_message: "Please fix failing doc-tests in output above.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::DOC_CHECK) {
        // Check that building docs work and does not emit warnings
        // std::env::set_var("RUSTDOCFLAGS", "-D warnings");

        let mut args = vec![
            "--workspace",
            "--all-features",
            "--no-deps",
            "--document-private-items",
        ];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--keep-going");
        }

        test_suite.insert(
            Check::DOC_CHECK,
            vec![CITest {
                command: cmd!(sh, "cargo doc {args...}"),
                failure_message: "Please fix doc warnings in output above.",
                subdir: None,
                env_vars: vec![("RUSTDOCFLAGS", "-D warnings")],
            }],
        );
    }

    if checks.contains(Check::BENCH_CHECK) {
        // Check that benches are building
        let mut args = vec!["--benches", "--target-dir", "../target"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--keep-going");
        }

        test_suite.insert(
            Check::BENCH_CHECK,
            vec![CITest {
                command: cmd!(sh, "cargo check {args...}"),
                failure_message: "Failed to check the benches.",
                subdir: Some("benches"),
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::EXAMPLE_CHECK) {
        // Build examples and check they compile
        let mut args = vec!["--workspace", "--examples"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--keep-going");
        }

        test_suite.insert(
            Check::EXAMPLE_CHECK,
            vec![CITest {
                command: cmd!(sh, "cargo check {args...}"),
                failure_message: "Please fix compiler errors for examples in output above.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::COMPILE_CHECK) {
        // Build bevy and check that it compiles
        let mut args = vec!["--workspace"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--keep-going");
        }

        test_suite.insert(
            Check::COMPILE_CHECK,
            vec![CITest {
                command: cmd!(sh, "cargo check {args...}"),
                failure_message: "Please fix compiler errors in output above.",
                subdir: None,
                env_vars: vec![],
            }],
        );
    }

    if checks.contains(Check::CFG_CHECK) {
        // Check cfg and imports
        let mut args = vec!["-Zcheck-cfg", "--workspace"];
        if flags.contains(Flag::KEEP_GOING) {
            args.push("--keep-going");
        }

        test_suite.insert(
            Check::CFG_CHECK,
            vec![CITest {
                command: cmd!(sh, "cargo +nightly check {args...}"),
                failure_message: "Please fix failing cfg checks in output above.",
                subdir: None,
                env_vars: vec![("RUSTFLAGS", "-D warnings")],
            }],
        );
    }

    // Actually run the tests:

    let mut failed_checks: Check = Check::empty();
    let mut failure_message: String = String::new();

    // In KEEP_GOING-mode, we save all errors until the end; otherwise, we just
    // panic with the given message for test failure.
    fn fail(
        current_check: Check,
        failure_message: &'static str,
        failed_checks: &mut Check,
        existing_fail_message: &mut String,
        flags: &Flag,
    ) {
        if flags.contains(Flag::KEEP_GOING) {
            failed_checks.insert(current_check);
            if !existing_fail_message.is_empty() {
                existing_fail_message.push('\n');
            }
            existing_fail_message.push_str(failure_message);
        } else {
            panic!("{failure_message}");
        }
    }

    for (check, battery) in test_suite.into_iter() {
        for ci_test in battery {
            // Ensure that necessary environment variables are set
            for (k, v) in ci_test.env_vars {
                std::env::set_var(k, v);
            }

            // If the CI test is to be executed in a subdirectory, we move there before running the command
            let _hook = ci_test.subdir.map(|path| sh.push_dir(path));

            // Actually run the test
            if ci_test.command.run().is_err() {
                fail(
                    check,
                    ci_test.failure_message,
                    &mut failed_checks,
                    &mut failure_message,
                    &flags,
                )
            }
            // ^ This must run while `_hook` is in scope; it is dropped at the end of the inner loop iteration.
        }
    }

    if !failed_checks.is_empty() {
        panic!(
            "One or more CI checks failed.\n
            {failure_message}"
        );
    }
}
