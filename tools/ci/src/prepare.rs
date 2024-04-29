use bitflags::bitflags;

/// Trait for preparing a subcommand to be run.
pub trait Prepare {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>>;
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Flag: u32 {
        /// Forces certain checks to continue running even if they hit an error.
        const KEEP_GOING = 1 << 0;
    }
}

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
}

impl<'a> PreparedCommand<'a> {
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
        }
    }

    pub fn with_subdir(mut self, subdir: &'static str) -> Self {
        self.subdir = Some(subdir);
        self
    }

    pub fn with_env_var(mut self, key: &'static str, value: &'static str) -> Self {
        self.env_vars.push((key, value));
        self
    }
}
