use crate::commands::prepare::{Flag, Prepare, PreparedCommand};
use crate::commands::subcommands;
use argh::FromArgs;

/// The CI command line tool for Bevy.
#[derive(FromArgs)]
pub(crate) struct CI {
    #[argh(subcommand)]
    command: Option<Commands>,
    /// continue running commands even if one fails
    #[argh(switch)]
    keep_going: bool,
}

impl CI {
    /// Runs the specified commands or all commands if none are specified.
    ///
    /// When run locally, results may differ from actual CI runs triggered by `.github/workflows/ci.yml`.
    /// This is because the official CI runs latest stable, while local runs use whatever the default Rust is locally.
    pub fn run(self) {
        let sh = xshell::Shell::new().unwrap();

        let prepared_commands = self.prepare(&sh);

        let mut failures = vec![];

        for command in prepared_commands {
            // If the CI test is to be executed in a subdirectory, we move there before running the command.
            // This will automatically move back to the original directory once dropped.
            let _subdir_hook = command.subdir.map(|path| sh.push_dir(path));

            if command.command.envs(command.env_vars).run().is_err() {
                let name = command.name;
                let message = command.failure_message;
                if self.keep_going {
                    failures.push(format!("- {name}: {message}"));
                } else {
                    failures.push(format!("{name}: {message}"));
                }
            }
        }

        if !failures.is_empty() {
            let failures = failures.join("\n");
            panic!(
                "One or more CI commands failed:\n\
                {failures}"
            );
        }
    }

    fn prepare<'a>(&self, sh: &'a xshell::Shell) -> Vec<PreparedCommand<'a>> {
        let mut flags = Flag::empty();

        if self.keep_going {
            flags |= Flag::KEEP_GOING;
        }

        match &self.command {
            Some(command) => command.prepare(sh, flags),
            None => {
                // Note that we are running the subcommands directly rather than using any aliases
                let mut cmds = vec![];
                cmds.append(&mut subcommands::FormatCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::ClippyCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::TestCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::TestCheckCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::DocCheckCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::DocTestCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::CompileCheckCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::CfgCheckCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::CompileFailCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::BenchCheckCommand::default().prepare(sh, flags));
                cmds.append(&mut subcommands::ExampleCheckCommand::default().prepare(sh, flags));
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
    Lints(subcommands::LintsCommand),
    Doc(subcommands::DocCommand),
    Compile(subcommands::CompileCommand),
    // Actual subcommands
    Format(subcommands::FormatCommand),
    Clippy(subcommands::ClippyCommand),
    Test(subcommands::TestCommand),
    TestCheck(subcommands::TestCheckCommand),
    DocCheck(subcommands::DocCheckCommand),
    DocTest(subcommands::DocTestCommand),
    CompileCheck(subcommands::CompileCheckCommand),
    CfgCheck(subcommands::CfgCheckCommand),
    CompileFail(subcommands::CompileFailCommand),
    BenchCheck(subcommands::BenchCheckCommand),
    ExampleCheck(subcommands::ExampleCheckCommand),
}

impl Prepare for Commands {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        match self {
            Commands::Lints(subcommand) => subcommand.prepare(sh, flags),
            Commands::Doc(subcommand) => subcommand.prepare(sh, flags),
            Commands::Compile(subcommand) => subcommand.prepare(sh, flags),

            Commands::Format(subcommand) => subcommand.prepare(sh, flags),
            Commands::Clippy(subcommand) => subcommand.prepare(sh, flags),
            Commands::Test(subcommand) => subcommand.prepare(sh, flags),
            Commands::TestCheck(subcommand) => subcommand.prepare(sh, flags),
            Commands::DocCheck(subcommand) => subcommand.prepare(sh, flags),
            Commands::DocTest(subcommand) => subcommand.prepare(sh, flags),
            Commands::CompileCheck(subcommand) => subcommand.prepare(sh, flags),
            Commands::CfgCheck(subcommand) => subcommand.prepare(sh, flags),
            Commands::CompileFail(subcommand) => subcommand.prepare(sh, flags),
            Commands::BenchCheck(subcommand) => subcommand.prepare(sh, flags),
            Commands::ExampleCheck(subcommand) => subcommand.prepare(sh, flags),
        }
    }
}
