use crate::commands;
use argh::FromArgs;

/// The CI command line tool for Bevy.
#[derive(FromArgs)]
pub struct CI {
    #[argh(subcommand)]
    command: Commands,

    /// continue running commands even if one fails
    #[argh(switch)]
    keep_going: bool,
}

impl CI {
    /// Runs the specified commands or all commands if none are specified.
    ///
    /// When run locally, results may differ from actual CI runs triggered by `.github/workflows/ci.yml`.
    /// This is usually related to differing toolchains and configuration.
    pub fn run(self) -> Result<(), ()> {
        match self.command {
            Commands::Lints(lints) => lints.run(self.keep_going),
            Commands::Doc(doc) => doc.run(self.keep_going),
            Commands::Compile(compile) => compile.run(self.keep_going),
            Commands::Format(format) => format.run(),
            Commands::Clippy(clippy) => clippy.run(),
            Commands::Test(test) => test.run(self.keep_going),
            Commands::TestCheck(test_check) => test_check.run(),
            Commands::DocCheck(doc_check) => doc_check.run(),
            Commands::DocTest(doc_test) => doc_test.run(self.keep_going),
            Commands::CompileCheck(compile_check) => compile_check.run(),
            Commands::CfgCheck(cfg_check) => cfg_check.run(),
            Commands::CompileFail(compile_fail) => compile_fail.run(self.keep_going),
            Commands::BenchCheck(bench_check) => bench_check.run(),
            Commands::ExampleCheck(example_check) => example_check.run(),
        }
    }
}

/// The subcommands that can be run by the CI script.
#[derive(FromArgs)]
#[argh(subcommand)]
enum Commands {
    // Aliases (subcommands that run other subcommands)
    Lints(commands::LintsCommand),
    Doc(commands::DocCommand),
    Compile(commands::CompileCommand),
    // Actual subcommands
    Format(commands::FormatCommand),
    Clippy(commands::ClippyCommand),
    Test(commands::TestCommand),
    TestCheck(commands::TestCheckCommand),
    DocCheck(commands::DocCheckCommand),
    DocTest(commands::DocTestCommand),
    CompileCheck(commands::CompileCheckCommand),
    CfgCheck(commands::CfgCheckCommand),
    CompileFail(commands::CompileFailCommand),
    BenchCheck(commands::BenchCheckCommand),
    ExampleCheck(commands::ExampleCheckCommand),
}
