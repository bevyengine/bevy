use crate::{
    args::Args,
    commands,
    prepare::{Prepare, PreparedCommand},
};
use argh::FromArgs;

/// The CI command line tool for Bevy.
#[derive(FromArgs)]
pub struct CI {
    #[argh(subcommand)]
    command: Option<Commands>,

    /// continue running commands even if one fails
    #[argh(switch)]
    pub(crate) keep_going: bool,

    /// parallelism of `cargo test`
    #[argh(option)]
    pub(crate) test_threads: Option<usize>,

    /// number of build jobs
    #[argh(option)]
    pub(crate) build_jobs: Option<usize>,
}

impl CI {
    /// Runs the specified commands or all commands if none are specified.
    ///
    /// When run locally, results may differ from actual CI runs triggered by `.github/workflows/ci.yml`.
    /// This is usually related to differing toolchains and configuration.
    pub fn run(self) {
        let sh = xshell::Shell::new().unwrap();
        let prepared_commands = self.prepare(&sh);

        let mut failures = vec![];

        for command in prepared_commands {
            // If the CI test is to be executed in a subdirectory, we move there before running the command.
            // This will automatically move back to the original directory once dropped.
            let _subdir_hook = command.subdir.map(|path| sh.push_dir(path));

            // Execute each command, checking if it returned an error.
            if command.command.envs(command.env_vars).run().is_err() {
                let name = command.name;
                let message = command.failure_message;

                if self.keep_going {
                    // We use bullet points here because there can be more than one error.
                    failures.push(format!("- {name}: {message}"));
                } else {
                    failures.push(format!("{name}: {message}"));
                    break;
                }
            }
        }

        // Log errors at the very end.
        if !failures.is_empty() {
            let failures = failures.join("\n");

            panic!(
                "One or more CI commands failed:\n\
                {failures}"
            );
        }
    }

    fn prepare<'a>(&self, sh: &'a xshell::Shell) -> Vec<PreparedCommand<'a>> {
        let args = self.into();
        match &self.command {
            Some(command) => command.prepare(sh, args),
            None => {
                // Note that we are running the subcommands directly rather than using any aliases
                let mut cmds = vec![];
                cmds.append(&mut commands::FormatCommand::default().prepare(sh, args));
                cmds.append(&mut commands::ClippyCommand::default().prepare(sh, args));
                cmds.append(&mut commands::TestCommand::default().prepare(sh, args));
                cmds.append(&mut commands::TestCheckCommand::default().prepare(sh, args));
                cmds.append(&mut commands::IntegrationTestCommand::default().prepare(sh, args));
                cmds.append(
                    &mut commands::IntegrationTestCheckCommand::default().prepare(sh, args),
                );
                cmds.append(
                    &mut commands::IntegrationTestCleanCommand::default().prepare(sh, args),
                );
                cmds.append(&mut commands::DocCheckCommand::default().prepare(sh, args));
                cmds.append(&mut commands::DocTestCommand::default().prepare(sh, args));
                cmds.append(&mut commands::CompileCheckCommand::default().prepare(sh, args));
                cmds.append(&mut commands::CompileFailCommand::default().prepare(sh, args));
                cmds.append(&mut commands::BenchCheckCommand::default().prepare(sh, args));
                cmds.append(&mut commands::ExampleCheckCommand::default().prepare(sh, args));

                cmds
            }
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
    IntegrationTest(commands::IntegrationTestCommand),
    IntegrationTestCheck(commands::IntegrationTestCheckCommand),
    IntegrationTestClean(commands::IntegrationTestCleanCommand),
    DocCheck(commands::DocCheckCommand),
    DocTest(commands::DocTestCommand),
    CompileCheck(commands::CompileCheckCommand),
    CompileFail(commands::CompileFailCommand),
    BenchCheck(commands::BenchCheckCommand),
    ExampleCheck(commands::ExampleCheckCommand),
}

impl Prepare for Commands {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        match self {
            Commands::Lints(subcommand) => subcommand.prepare(sh, args),
            Commands::Doc(subcommand) => subcommand.prepare(sh, args),
            Commands::Compile(subcommand) => subcommand.prepare(sh, args),

            Commands::Format(subcommand) => subcommand.prepare(sh, args),
            Commands::Clippy(subcommand) => subcommand.prepare(sh, args),
            Commands::Test(subcommand) => subcommand.prepare(sh, args),
            Commands::TestCheck(subcommand) => subcommand.prepare(sh, args),
            Commands::IntegrationTest(subcommand) => subcommand.prepare(sh, args),
            Commands::IntegrationTestCheck(subcommand) => subcommand.prepare(sh, args),
            Commands::IntegrationTestClean(subcommand) => subcommand.prepare(sh, args),
            Commands::DocCheck(subcommand) => subcommand.prepare(sh, args),
            Commands::DocTest(subcommand) => subcommand.prepare(sh, args),
            Commands::CompileCheck(subcommand) => subcommand.prepare(sh, args),
            Commands::CompileFail(subcommand) => subcommand.prepare(sh, args),
            Commands::BenchCheck(subcommand) => subcommand.prepare(sh, args),
            Commands::ExampleCheck(subcommand) => subcommand.prepare(sh, args),
        }
    }
}
