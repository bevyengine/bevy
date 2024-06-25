use std::io::{stderr, stdout, Write};

use bitflags::bitflags;
use xshell::Shell;

use crate::json::JsonCommandOutput;

/// Trait for preparing a subcommand to be run.
pub trait Prepare {
    /// A method that returns a list of [`PreparedCommand`]s to be run for a given shell and flags.
    ///
    /// # Example
    ///
    /// ```
    /// # use crate::{Flag, Prepare, PreparedCommand};
    /// # use argh::FromArgs;
    /// # use xshell::Shell;
    /// #
    /// #[derive(FromArgs)]
    /// #[argh(subcommand, name = "check")]
    /// struct CheckCommand {}
    ///
    /// impl Prepare for CheckCommand {
    ///     fn prepare<'a>(&self, sh: &'a Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
    ///         vec![PreparedCommand::new::<Self>(
    ///             cmd!(sh, "cargo check --workspace"),
    ///             "Please fix linter errors",
    ///         )]
    ///     }
    /// }
    /// ```
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>>;
}

bitflags! {
    /// Flags that modify how commands are run.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Flag: u32 {
        /// Forces certain checks to continue running even if they hit an error.
        const KEEP_GOING = 1 << 0;
    }
}

/// A command with associated metadata, created from a command that implements [`Prepare`].
#[derive(Debug)]
pub struct PreparedCommand<'a> {
    /// The name of the command.
    pub name: &'static str,

    /// The command to execute
    pub command: xshell::Cmd<'a>,

    /// The message to display if the test command fails
    pub failure_message: &'static str,

    /// The subdirectory path to run the test command within
    pub subdir: Option<&'static str>,

    /// Environment variables that need to be set before the test runs
    pub env_vars: Vec<(&'static str, &'static str)>,

    /// The command outputs cargo formatted json
    pub emit_json: bool,
}

impl<'a> PreparedCommand<'a> {
    /// Creates a new [`PreparedCommand`] from a [`Cmd`] and a failure message.
    ///
    /// The other fields of [`PreparedCommand`] are filled in with their default values.
    ///
    /// For more information about creating a [`Cmd`], please see the [`cmd!`](xshell::cmd) macro.
    ///
    /// [`Cmd`]: xshell::Cmd
    pub fn new<T: argh::SubCommand>(
        command: xshell::Cmd<'a>,
        failure_message: &'static str,
    ) -> Self {
        Self {
            command,
            name: T::COMMAND.name,
            failure_message,
            subdir: None,
            env_vars: vec![],
            emit_json: false,
        }
    }

    /// A builder that overwrites the current sub-directory with a new value.
    pub fn with_subdir(mut self, subdir: &'static str) -> Self {
        self.subdir = Some(subdir);
        self
    }

    /// A builder that adds a new environmental variable to the list.
    pub fn with_env_var(mut self, key: &'static str, value: &'static str) -> Self {
        self.env_vars.push((key, value));
        self
    }

    /// A builder that controls whetever this command outputs json
    pub fn with_json(mut self, emit_json: bool) -> Self {
        self.emit_json = emit_json;
        self
    }

    /// Runs this command
    ///
    /// If the command otputs json will return a [`JsonCommandOutput`]
    pub fn run(
        &mut self,
        shell: &Shell,
    ) -> Result<Option<JsonCommandOutput>, Option<JsonCommandOutput>> {
        // If the CI test is to be executed in a subdirectory, we move there before running the command.
        // This will automatically move back to the original directory once dropped.
        let _subdir_hook = self.subdir.map(|path| shell.push_dir(path));

        // For json outputting commands we want to read stdout even when things fail.
        self.command.set_ignore_status(true);

        let output = self.command.output().map_err(|_| None)?;

        let json = if self.emit_json {
            JsonCommandOutput::from_cargo_output(output.stdout, self.name.to_string())
        } else {
            stdout().write_all(&output.stdout).unwrap();
            None
        };

        stderr().write_all(&output.stderr).unwrap();

        if output.status.success() {
            Ok(json)
        } else {
            eprintln!("{}", self.failure_message);
            Err(json)
        }
    }
}
