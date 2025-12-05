use argh::{FromArgValue, FromArgs};
use xshell::Shell;

use crate::args::Args;

/// Decides which jobs to run
#[derive(FromArgs)]
#[argh(subcommand, name = "what-to-run")]
pub struct WhatToRunCommand {
    /// which event triggered this run
    #[argh(option)]
    trigger: Trigger,

    /// which branch to diff against
    #[argh(option)]
    head: String,

    /// select tasks for a specific version of rust
    #[argh(option)]
    #[expect(dead_code)]
    rust_version: RustVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustVersion {
    Stable,
    Beta,
    Nightly,
}

impl FromArgValue for RustVersion {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value {
            "stable" => Ok(RustVersion::Stable),
            "beta" => Ok(RustVersion::Beta),
            "nightly" => Ok(RustVersion::Nightly),
            _ => Err(format!("Unknown rust version: {}", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    Schedule,
    MergeQueue,
    ChangeRequest,
    PushToBranch,
}

impl FromArgValue for Trigger {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        match value {
            "schedule" => Ok(Trigger::Schedule),
            "merge_group" => Ok(Trigger::MergeQueue),
            "pull_request" => Ok(Trigger::ChangeRequest),
            "push" => Ok(Trigger::PushToBranch),
            _ => Err(format!("Unknown trigger: {}", value)),
        }
    }
}

impl WhatToRunCommand {
    pub fn run(&self, _sh: &Shell, _args: Args) {
        let _diff = match self.trigger {
            Trigger::Schedule | Trigger::PushToBranch => vec![],
            Trigger::ChangeRequest | Trigger::MergeQueue => get_diff(&self.head),
        };

        // TODO: filter jobs to run based on diff, trigger and rust version
        let mut jobs = Vec::new();
        jobs.push(r#""cargo run -p ci -- test""#);
        jobs.push(r#""cargo run -p ci -- lints""#);

        println!("[{}]", jobs.join(", "))
    }
}

fn get_diff(_head: &str) -> Vec<String> {
    // TODO: Implement diff logic between local state and head branch
    vec![]
}
