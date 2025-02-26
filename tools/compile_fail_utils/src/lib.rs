use std::{
    env,
    path::{Path, PathBuf},
};

// Re-export ui_test so all the tests use the same version.
pub use ui_test;

use ui_test::{
    color_eyre::eyre::eyre,
    default_file_filter, default_per_file_config,
    dependencies::DependencyBuilder,
    run_tests_generic,
    spanned::Spanned,
    status_emitter::{Gha, StatusEmitter, Text},
    Args, Config,
};

/// Use this instead of hand rolling configs.
///
/// `root_dir` is the directory your tests are contained in. Needs to be a path from crate root.
/// This config will build dependencies and will assume that the cargo manifest is placed at the
/// current working directory.
fn basic_config(root_dir: impl Into<PathBuf>, args: &Args) -> ui_test::Result<Config> {
    let root_dir = root_dir.into();

    match root_dir.try_exists() {
        Ok(true) => { /* success */ }
        Ok(false) => {
            return Err(eyre!("path does not exist: {}", root_dir.display()));
        }
        Err(error) => {
            return Err(eyre!(
                "failed to read path: {} ({})",
                root_dir.display(),
                error
            ));
        }
    }

    let mut config = Config {
        bless_command: Some(
            "`cargo test` with the BLESS environment variable set to any non empty value"
                .to_string(),
        ),
        output_conflict_handling: if env::var_os("BLESS").is_some() {
            ui_test::bless_output_files
        } else {
            ui_test::error_on_output_conflict
        },
        ..Config::rustc(root_dir)
    };

    config.with_args(args);

    // Window paths (cargo should already be doing this, but just in case).
    config.stderr_filter(r"\\", "/");
    // Replace line and column numbers (regex patterns shamelessly stolen from miri).
    config.stderr_filter(r"\.rs:[0-9]+:[0-9]+(: [0-9]+:[0-9]+)?", ".rs:LL:CC");
    // Replace stdlib path (stolen from miri again).
    config.stderr_filter(
        r"[^ \n`]*/(?:rust[^/]*|checkout|[0-9a-fA-F]*)/library/",
        "RUSTLIB/",
    );
    // Replace long type file names since they contain random numbers
    config.stderr_filter(r"[\p{L}\p{N}_]+\.long-type-\d+\.txt", "long-type.sr'");
    // The number of spaces in diagnostics isn't consistent across platforms
    config.stderr_filter(r"\n +-->", "\n  -->");
    config.stderr_filter(r"\n\d+ +\|", "\nLL |");
    config.stderr_filter(r"\n +\|", "\n   |");

    let bevy_root = "..";

    // Don't leak contributor filesystem paths
    config.path_stderr_filter(Path::new(bevy_root), b"BEVY_ROOT");

    // Manually insert @aux-build:<dep> comments into test files. This needs to
    // be done to build and link dependencies. Dependencies will be pulled from a
    // Cargo.toml file.
    config.comment_defaults.base().custom.insert(
        "dependencies",
        Spanned::dummy(vec![Box::new(DependencyBuilder::default())]),
    );

    Ok(config)
}

/// Runs ui tests for a single directory.
///
/// `root_dir` is the directory your tests are contained in. Needs to be a path from crate root.
pub fn test(test_name: impl Into<String>, test_root: impl Into<PathBuf>) -> ui_test::Result<()> {
    test_multiple(test_name, [test_root])
}

/// Run ui tests with the given config
pub fn test_with_config(test_name: impl Into<String>, config: Config) -> ui_test::Result<()> {
    test_with_multiple_configs(test_name, [Ok(config)])
}

/// Runs ui tests for a multiple directories.
///
/// `root_dirs` paths need to come from crate root.
pub fn test_multiple(
    test_name: impl Into<String>,
    test_roots: impl IntoIterator<Item = impl Into<PathBuf>>,
) -> ui_test::Result<()> {
    let args = Args::test()?;

    let configs = test_roots.into_iter().map(|root| basic_config(root, &args));

    test_with_multiple_configs(test_name, configs)
}

/// Run ui test with the given configs.
///
/// Tests for configs are run in parallel.
pub fn test_with_multiple_configs(
    test_name: impl Into<String>,
    configs: impl IntoIterator<Item = ui_test::Result<Config>>,
) -> ui_test::Result<()> {
    let configs = configs
        .into_iter()
        .collect::<ui_test::Result<Vec<Config>>>()?;

    let emitter: Box<dyn StatusEmitter + Send> = if env::var_os("CI").is_some() {
        Box::new((
            Text::verbose(),
            Gha::<true> {
                name: test_name.into(),
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
