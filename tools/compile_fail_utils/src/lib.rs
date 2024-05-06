use std::{
    env,
    path::{Path, PathBuf},
};

// Re-export ui_test so all the tests use the same version.
pub use ui_test;

use ui_test::{
    default_file_filter, default_per_file_config,
    dependencies::DependencyBuilder,
    run_tests_generic,
    spanned::Spanned,
    status_emitter::{Gha, StatusEmitter, Text},
    Args, Config, OutputConflictHandling,
};

/// Use this instead of hand rolling configs.
///
/// `root_dir` is the directory your tests are contained in. Needs to be a path from crate root.
/// This config will build dependencies and will assume that the cargo manifest is placed at the 
/// current working directory.
fn basic_config(root_dir: impl Into<PathBuf>, args: &Args) -> Config {
    let mut config = Config {
        bless_command: Some(
            "`cargo test` with the BLESS environment variable set to any non empty value"
                .to_string(),
        ),
        output_conflict_handling: if env::var_os("BLESS").is_some() {
            OutputConflictHandling::Bless
        } else {
            // stderr output changes between rust versions so we just rely on annotations
            OutputConflictHandling::Ignore
        },
        ..Config::rustc(root_dir)
    };

    config.with_args(args);

    let bevy_root = "..";

    // Don't leak contributor filesystem paths
    config.path_stderr_filter(Path::new(bevy_root), b"$BEVY_ROOT");
    config.path_stderr_filter(Path::new(env!("RUSTUP_HOME")), b"$RUSTUP_HOME");

    // ui_test doesn't compile regex with perl character classes.
    // \pL = unicode class for letters, \pN = unicode class for numbers
    config.stderr_filter(r"\/home\/[\pL\pN_@#\-\. ]+", "$HOME");
    // Paths in .stderr seem to always be normalized to use /. Handle both anyway.
    config.stderr_filter(
        r"[a-zA-Z]:(?:\\|\/)users(?:\\|\/)[\pL\pN_@#\-\. ]+", // NOTE: [\pL\pN_@#\-\. ] is a poor attempt at handling usernames
        "$HOME",
    );

    // Manually insert @aux-build:<dep> comments into test files. This needs to 
    // be done to build and link dependencies. Dependencies will be pulled from a
    // Cargo.toml file.
    config.comment_defaults.base().custom.insert(
        "dependencies",
        Spanned::dummy(vec![Box::new(DependencyBuilder::default())]),
    );

    config
}

/// Runs ui tests for a single directory.
///
/// `root_dir` is the directory your tests are contained in. Needs to be a path from crate root.
pub fn test(test_root: impl Into<PathBuf>) -> ui_test::Result<()> {
    test_multiple([test_root])
}

/// Run ui tests with the given config
pub fn test_with_config(config: Config) -> ui_test::Result<()> {
    test_with_multiple_configs([config])
}

/// Runs ui tests for a multiple directories.
///
/// `root_dirs` paths need to come from crate root.
pub fn test_multiple(
    test_roots: impl IntoIterator<Item = impl Into<PathBuf>>,
) -> ui_test::Result<()> {
    let args = Args::test()?;

    let configs = test_roots.into_iter().map(|root| basic_config(root, &args));

    test_with_multiple_configs(configs)
}

/// Run ui test with the given configs.
///
/// Tests for configs are run in parallel.
pub fn test_with_multiple_configs(
    configs: impl IntoIterator<Item = Config>,
) -> ui_test::Result<()> {
    let configs = configs.into_iter().collect();

    let emitter: Box<dyn StatusEmitter + Send> = if env::var_os("CI").is_some() {
        Box::new((
            Text::verbose(),
            Gha::<true> {
                name: env!("CARGO_PKG_NAME").to_string(),
            },
        ))
    } else {
        Box::new(Text::quiet())
    };

    run_tests_generic(
        configs,
        default_file_filter,
        default_per_file_config,
        emitter,
    )
}
